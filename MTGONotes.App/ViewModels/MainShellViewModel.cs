using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;

namespace MTGONotes.App.ViewModels;

public sealed class MainShellViewModel : ViewModel
{
    private string _liveStatus = string.Empty;

    public MainShellViewModel(AppHost host)
    {
        Encounter = new EncounterViewModel(host);
        History = new HistoryViewModel(host);
        Settings = new SettingsViewModel(host);
        LiveStatus = Encounter.LiveStatus;
    }

    public EncounterViewModel Encounter { get; }

    public HistoryViewModel History { get; }

    public SettingsViewModel Settings { get; }

    public string LiveStatus
    {
        get => _liveStatus;
        set => SetProperty(ref _liveStatus, value);
    }

    public void Apply(OverlayView view)
    {
        Encounter.Apply(view);
        LiveStatus = Encounter.LiveStatus;
        History.NotifyDisclosure(view.Phase);
    }
}
