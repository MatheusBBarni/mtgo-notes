using Microsoft.UI.Xaml;
using MTGONotes.App.Host;

namespace MTGONotes.App;

public partial class App : Application
{
    private AppHost? _host;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _host = new AppHost();
        _host.Start();
    }
}
