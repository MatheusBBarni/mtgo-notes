using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.App.ViewModels;

public sealed partial class EncounterViewModel : ObservableObject
{
    private readonly AppHost _host;

    public EncounterViewModel(AppHost host) => _host = host;

    [ObservableProperty]
    public partial string Handle { get; set; } = string.Empty;

    [ObservableProperty]
    public partial string PhaseTitle { get; set; } = PresentationText.FormatEncounterHeading(InternalPhase.Idle, null);

    [ObservableProperty]
    public partial string LiveStatus { get; set; } = string.Empty;

    [ObservableProperty]
    public partial string ConfirmedHandle { get; set; } = string.Empty;

    [ObservableProperty]
    public partial string StatusMessage { get; set; } = string.Empty;

    [ObservableProperty]
    public partial bool IsStatusOpen { get; set; }

    [ObservableProperty]
    public partial bool IsStatusError { get; set; }

    [ObservableProperty]
    public partial bool IsLivePaused { get; set; }

    [ObservableProperty]
    public partial bool HasConfirmedOpponent { get; set; }

    [ObservableProperty]
    public partial string PauseLabel { get; set; } = "Pause live attach";

    public ObservableCollection<ObservationItem> Notes { get; } = [];

    public bool HasNotes => Notes.Count > 0;

    public void Apply(OverlayView view)
    {
        PhaseTitle = PresentationText.FormatEncounterHeading(view.Phase, view.ConfirmedHandle);
        ConfirmedHandle = view.ConfirmedHandle ?? string.Empty;
        HasConfirmedOpponent = view.ConfirmedHandle is not null;
        LiveStatus = ResolveLiveStatus();
        IsLivePaused = _host.Session.DetectionPaused;
        PauseLabel = IsLivePaused ? "Resume live attach" : "Pause live attach";
        Notes.Clear();
        foreach (var note in view.CurrentObservations)
        {
            Notes.Add(new ObservationItem(note.Id, note.Text));
        }

        OnPropertyChanged(nameof(HasNotes));
        FinishEncounterCommand.NotifyCanExecuteChanged();
    }

    [RelayCommand(CanExecute = nameof(CanConfirm))]
    private void ConfirmOpponent()
    {
        var result = _host.Session.EnterOpponent(Handle);
        if (result.IsSuccess)
        {
            SetStatus("Opponent confirmed.", isError: false);
        }
        else
        {
            SetStatus(result.Error!.Value.ToAppError().Message, isError: true);
        }

        Apply(_host.Session.CurrentView);
    }

    [RelayCommand(CanExecute = nameof(CanFinish))]
    private void FinishEncounter()
    {
        var result = _host.Session.FinishEncounter();
        if (result.IsSuccess)
        {
            SetStatus("Encounter finished.", isError: false);
        }
        else
        {
            SetStatus(result.Error!.Value.ToAppError().Message, isError: true);
        }

        Apply(_host.Session.CurrentView);
    }

    [RelayCommand]
    private void ToggleLivePause()
    {
        var paused = !_host.Session.DetectionPaused;
        _ = _host.Session.PauseDetection(paused);
        Apply(_host.Session.CurrentView);
        SetStatus(paused ? "Live attach paused." : "Live attach resumed.", isError: false);
    }

    partial void OnHandleChanged(string value) => ConfirmOpponentCommand.NotifyCanExecuteChanged();

    partial void OnHasConfirmedOpponentChanged(bool value) =>
        FinishEncounterCommand.NotifyCanExecuteChanged();

    private bool CanConfirm() => !string.IsNullOrWhiteSpace(Handle);

    private bool CanFinish() => HasConfirmedOpponent;

    private void SetStatus(string message, bool isError)
    {
        StatusMessage = message;
        IsStatusError = isError;
        IsStatusOpen = !string.IsNullOrWhiteSpace(message);
    }

    private string ResolveLiveStatus()
    {
        if (_host.StartupWarning is { } warning)
        {
            return $"Notebook unavailable: {warning}";
        }

        return _host.Live.IsAttached
            ? "Live attach: connected"
            : "Live attach: waiting for a logged-in MTGO client";
    }
}
