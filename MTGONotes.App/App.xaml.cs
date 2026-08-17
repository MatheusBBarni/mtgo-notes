using Microsoft.UI.Xaml;
using MTGONotes.App.Host;
using MTGONotes.App.Native;

namespace MTGONotes.App;

public partial class App : Application
{
    private AppHost? _host;

    public App()
    {
        InitializeComponent();
        SingleInstance = SingleInstance.Claim();
    }

    public SingleInstance? SingleInstance { get; }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        if (SingleInstance is not { IsOwner: true })
        {
            Exit();
            return;
        }

        _host = new AppHost();
        _host.Start();
    }
}
