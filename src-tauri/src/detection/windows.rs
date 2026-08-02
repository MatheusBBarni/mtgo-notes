use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM};

use tauri::Manager;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GA_ROOT, GetAncestor, GetClassNameW, GetWindowTextLengthW, GetWindowTextW,
    IsIconic, IsWindow, IsWindowVisible,
};
use windows::core::BOOL;

use crate::detection::{
    AuthorizedWindow, ContextField, DetectionRuntime, EvidenceInput, EvidenceProvenance,
};
use crate::domain::{RepoError, UtcMillis};

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const MAX_UIA_ELEMENTS: usize = 512;

pub fn validate_selected_window(window: &AuthorizedWindow) -> Result<(), RepoError> {
    let raw = isize::try_from(window.native_handle).map_err(|_| RepoError::WindowNotFound)?;
    let hwnd = HWND(raw as *mut core::ffi::c_void);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool()
            || !IsWindowVisible(hwnd).as_bool()
            || IsIconic(hwnd).as_bool()
            || GetAncestor(hwnd, GA_ROOT).0 != hwnd.0
        {
            return Err(RepoError::WindowNotFound);
        }
        let (class_name, visible_title) = describe_window(hwnd)?;
        if class_name != window.class_name
            || visible_title != window.visible_title
            || !visible_title.to_ascii_lowercase().contains("magic online")
        {
            return Err(RepoError::WindowNotFound);
        }
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|_| RepoError::ProviderUnavailable)?;
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
            .map_err(|_| RepoError::ProviderUnavailable)?;
        automation
            .ElementFromHandle(hwnd)
            .map_err(|_| RepoError::ProviderUnavailable)?;
    }
    Ok(())
}

pub fn list_visible_mtgo_windows() -> Result<Vec<AuthorizedWindow>, RepoError> {
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return true.into();
            }
            let Ok((class_name, title)) = describe_window(hwnd) else {
                return true.into();
            };
            if !title.to_ascii_lowercase().contains("magic online") {
                return true.into();
            }
            let windows = &mut *(lparam.0 as *mut Vec<AuthorizedWindow>);
            windows.push(AuthorizedWindow {
                native_handle: hwnd.0 as usize as u64,
                class_name,
                visible_title: title,
                selected_at: UtcMillis::now().get(),
                visible: true,
                minimized: false,
                usable_bounds: true,
            });
            true.into()
        }
    }

    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(visit),
            LPARAM(&mut windows as *mut Vec<AuthorizedWindow> as isize),
        )
        .map_err(|_| RepoError::ProviderUnavailable)?;
    }
    Ok(windows)
}

pub fn spawn_detection_worker(
    app: tauri::AppHandle,
    native_handle: u64,
    generation: u64,
) -> Result<(), RepoError> {
    std::thread::Builder::new()
        .name("mtgo-visible-detection".into())
        .spawn(move || run_detection_worker(app, native_handle, generation))
        .map(|_| ())
        .map_err(|_| RepoError::ProviderUnavailable)
}

fn run_detection_worker(app: tauri::AppHandle, native_handle: u64, generation: u64) {
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
    if !initialized {
        fail_closed(&app, generation);
        return;
    }

    let result = unsafe {
        let automation: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) {
            Ok(automation) => automation,
            Err(_) => {
                fail_closed(&app, generation);
                CoUninitialize();
                return;
            }
        };
        let walker = match automation.ControlViewWalker() {
            Ok(walker) => walker,
            Err(_) => {
                fail_closed(&app, generation);
                CoUninitialize();
                return;
            }
        };
        let started = Instant::now();
        let provider_session = crate::domain::EntityId::new().to_string();
        let mut sequence = 0_u64;
        let mut last_observed = BTreeMap::<ContextField, String>::new();

        loop {
            if !worker_is_current(&app, native_handle, generation) {
                break Ok(());
            }
            let hwnd = HWND(native_handle as usize as *mut core::ffi::c_void);
            let root = automation
                .ElementFromHandle(hwnd)
                .map_err(|_| RepoError::ProviderUnavailable)?;
            let locator_fields = app
                .state::<DetectionRuntime>()
                .engine
                .lock()
                .map_err(|_| RepoError::ProviderUnavailable)?
                .semantic_locator_fields();
            let mut evidence = collect_visible_evidence(&locator_fields, &walker, root)?;
            let observed_fields = evidence
                .iter()
                .map(|(field, _)| *field)
                .collect::<BTreeSet<_>>();
            if !observed_fields.contains(&ContextField::Phase) {
                evidence.push((ContextField::Phase, "gameplay".into()));
            }
            for (field, visible_text) in evidence {
                if last_observed.get(&field) == Some(&visible_text) {
                    continue;
                }
                last_observed.insert(field, visible_text.clone());
                sequence = sequence.saturating_add(1);
                dispatch_evidence(
                    &app,
                    EvidenceInput {
                        provider_session: provider_session.clone(),
                        generation,
                        sequence,
                        monotonic_ms: started.elapsed().as_millis() as u64,
                        field,
                        visible_text,
                        confidence: 1.0,
                        provenance: EvidenceProvenance::Uia,
                    },
                );
            }
            last_observed.retain(|field, _| {
                observed_fields.contains(field) || *field == ContextField::Phase
            });
            std::thread::sleep(POLL_INTERVAL);
        }
    };

    if result.is_err() {
        fail_closed(&app, generation);
    }
    unsafe { CoUninitialize() };
}

fn worker_is_current(app: &tauri::AppHandle, native_handle: u64, generation: u64) -> bool {
    app.state::<DetectionRuntime>()
        .engine
        .lock()
        .ok()
        .and_then(|engine| engine.worker_context())
        .is_some_and(|context| {
            context.native_handle == native_handle && context.generation == generation
        })
}

unsafe fn collect_visible_evidence(
    locator_fields: &BTreeMap<String, ContextField>,
    walker: &IUIAutomationTreeWalker,
    root: IUIAutomationElement,
) -> Result<Vec<(ContextField, String)>, RepoError> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    let mut visited = 0_usize;
    while let Some(element) = stack.pop() {
        visited = visited.saturating_add(1);
        if visited > MAX_UIA_ELEMENTS {
            break;
        }
        let automation_id = unsafe { element.CurrentAutomationId() }
            .ok()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let field = locator_fields.get(&automation_id).copied();
        if let Some(field) = field {
            let name = phase_fallback(field, &automation_id).or_else(|| {
                unsafe { element.CurrentName() }
                    .ok()
                    .map(|value| value.to_string())
                    .filter(|value| !value.trim().is_empty())
            });
            if let Some(name) = name {
                found.push((field, name));
            }
        }

        let mut child = unsafe { walker.GetFirstChildElement(&element) }.ok();
        while let Some(current) = child {
            child = unsafe { walker.GetNextSiblingElement(&current) }.ok();
            stack.push(current);
        }
    }
    Ok(found)
}

fn phase_fallback(field: ContextField, automation_id: &str) -> Option<String> {
    if field != ContextField::Phase {
        return None;
    }
    match automation_id {
        "SideboardingScene" => Some("sideboarding".into()),
        "MatchResults" => Some("results".into()),
        "DuelScene" => Some("gameplay".into()),
        _ => None,
    }
}

fn dispatch_evidence(app: &tauri::AppHandle, input: EvidenceInput) {
    let evidence = app
        .state::<DetectionRuntime>()
        .engine
        .lock()
        .ok()
        .and_then(|mut engine| engine.ingest(input).ok().flatten());
    let Some(evidence) = evidence else {
        return;
    };
    let encounters = app.state::<crate::commands::encounters::EncounterCommandRuntime>();
    match evidence.field {
        ContextField::Opponent => {
            if let Ok(Some(candidate)) = encounters.accept_detector_evidence(&evidence) {
                crate::commands::encounters::emit_opponent_candidate(app, candidate);
            }
        }
        ContextField::Phase => {
            let notebook = app.state::<crate::notebook::NotebookRuntime>();
            match encounters.apply_detector_evidence(
                &notebook.repository,
                &evidence,
                UtcMillis::now(),
            ) {
                Ok(Some(_)) => {
                    if crate::commands::encounters::emit_current_overlay(app, &notebook.repository)
                        .is_err()
                    {
                        crate::commands::encounters::emit_fail_closed_overlay(app);
                    }
                }
                Ok(None) => {}
                Err(_) => crate::commands::encounters::emit_fail_closed_overlay(app),
            }
        }
        ContextField::Format | ContextField::Game | ContextField::Result => {}
    }
}

fn fail_closed(app: &tauri::AppHandle, generation: u64) {
    let runtime = app.state::<DetectionRuntime>();
    let should_restrict = runtime.engine.lock().ok().is_some_and(|mut engine| {
        if engine
            .worker_context()
            .is_some_and(|context| context.generation == generation)
        {
            engine.revoke_window();
            true
        } else {
            false
        }
    });
    if !should_restrict {
        return;
    }
    let notebook = app.state::<crate::notebook::NotebookRuntime>();
    let _ = crate::commands::encounters::restrict_active_for_provider_interruption(
        &notebook.repository,
        "provider_unavailable",
        UtcMillis::now(),
    );
    crate::commands::encounters::emit_fail_closed_overlay(app);
}

unsafe fn describe_window(hwnd: HWND) -> Result<(String, String), RepoError> {
    let title_length = unsafe { GetWindowTextLengthW(hwnd) };
    if title_length <= 0 {
        return Err(RepoError::WindowNotFound);
    }
    let mut title = vec![0_u16; usize::try_from(title_length + 1).unwrap_or(0)];
    let copied = unsafe { GetWindowTextW(hwnd, &mut title) };
    if copied <= 0 {
        return Err(RepoError::WindowNotFound);
    }
    let visible_title =
        String::from_utf16_lossy(&title[..usize::try_from(copied).unwrap_or_default()]);
    let mut class_name = [0_u16; 256];
    let class_length = unsafe { GetClassNameW(hwnd, &mut class_name) };
    if class_length <= 0 {
        return Err(RepoError::WindowNotFound);
    }
    let class_name =
        String::from_utf16_lossy(&class_name[..usize::try_from(class_length).unwrap_or_default()]);
    Ok((class_name, visible_title))
}
