using System.Collections.ObjectModel;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.App.ViewModels;

public sealed class EncounterViewModel : ViewModel
{
    private readonly AppHost _host;
    private string _handle = string.Empty;
    private string _phaseTitle = PresentationText.FormatEncounterHeading(InternalPhase.Idle, null);
    private string _liveStatus = string.Empty;
    private string _confirmedHandle = string.Empty;
    private string _statusMessage = string.Empty;
    private bool _isStatusOpen;
    private bool _isStatusError;
    private bool _isLivePaused;
    private bool _hasConfirmedOpponent;
    private string _pauseLabel = "Pause live attach";

    public EncounterViewModel(AppHost host)
    {
        _host = host;
        ConfirmOpponentCommand = new RelayCommand(ConfirmOpponent, CanConfirm);
        FinishEncounterCommand = new RelayCommand(FinishEncounter, CanFinish);
        ToggleLivePauseCommand = new RelayCommand(ToggleLivePause);
    }

    public string Handle
    {
        get => _handle;
        set
        {
            if (SetProperty(ref _handle, value))
            {
                ConfirmOpponentCommand.NotifyCanExecuteChanged();
            }
        }
    }

    public string PhaseTitle
    {
        get => _phaseTitle;
        set => SetProperty(ref _phaseTitle, value);
    }

    public string LiveStatus
    {
        get => _liveStatus;
        set => SetProperty(ref _liveStatus, value);
    }

    public string ConfirmedHandle
    {
        get => _confirmedHandle;
        set => SetProperty(ref _confirmedHandle, value);
    }

    public string StatusMessage
    {
        get => _statusMessage;
        set => SetProperty(ref _statusMessage, value);
    }

    public bool IsStatusOpen
    {
        get => _isStatusOpen;
        set => SetProperty(ref _isStatusOpen, value);
    }

    public bool IsStatusError
    {
        get => _isStatusError;
        set => SetProperty(ref _isStatusError, value);
    }

    public bool IsLivePaused
    {
        get => _isLivePaused;
        set => SetProperty(ref _isLivePaused, value);
    }

    public bool HasConfirmedOpponent
    {
        get => _hasConfirmedOpponent;
        set
        {
            if (SetProperty(ref _hasConfirmedOpponent, value))
            {
                FinishEncounterCommand.NotifyCanExecuteChanged();
            }
        }
    }

    public string PauseLabel
    {
        get => _pauseLabel;
        set => SetProperty(ref _pauseLabel, value);
    }

    public ObservableCollection<ObservationItem> Notes { get; } = [];

    public bool HasNotes => Notes.Count > 0;

    public RelayCommand ConfirmOpponentCommand { get; }

    public RelayCommand FinishEncounterCommand { get; }

    public RelayCommand ToggleLivePauseCommand { get; }

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

    private void ConfirmOpponent()
    {
        var result = _host.Session.EnterOpponent(Handle);
        SetStatus(
            result.IsSuccess ? "Opponent confirmed." : result.Error!.Value.ToAppError().Message,
            isError: !result.IsSuccess);
        Apply(_host.Session.CurrentView);
    }

    private void FinishEncounter()
    {
        var result = _host.Session.FinishEncounter();
        SetStatus(
            result.IsSuccess ? "Encounter finished." : result.Error!.Value.ToAppError().Message,
            isError: !result.IsSuccess);
        Apply(_host.Session.CurrentView);
    }

    private void ToggleLivePause()
    {
        var paused = !_host.Session.DetectionPaused;
        _ = _host.Session.PauseDetection(paused);
        Apply(_host.Session.CurrentView);
        SetStatus(paused ? "Live attach paused." : "Live attach resumed.", isError: false);
    }

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
