using CommunityToolkit.Mvvm.ComponentModel;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;

namespace MTGONotes.App.ViewModels;

public sealed partial class MainShellViewModel : ObservableObject
{
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

    [ObservableProperty]
    public partial string LiveStatus { get; set; } = string.Empty;

    public void Apply(OverlayView view)
    {
        Encounter.Apply(view);
        LiveStatus = Encounter.LiveStatus;
        History.NotifyDisclosure(view.Phase);
    }
}
