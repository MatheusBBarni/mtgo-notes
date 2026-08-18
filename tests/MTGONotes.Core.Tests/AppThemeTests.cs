using MTGONotes.Core.Settings;

namespace MTGONotes.Core.Tests;

public sealed class AppThemeTests
{
    [Theory]
    [InlineData(null, AppTheme.System)]
    [InlineData("", AppTheme.System)]
    [InlineData("neon", AppTheme.System)]
    [InlineData("SYSTEM", AppTheme.System)]
    [InlineData(AppTheme.System, AppTheme.System)]
    [InlineData(AppTheme.Light, AppTheme.Light)]
    [InlineData(AppTheme.Dark, AppTheme.Dark)]
    public void Normalize_unknown_values_to_system(string? value, string expected)
    {
        Assert.Equal(expected, AppTheme.Normalize(value));
    }

    [Theory]
    [InlineData(AppTheme.System, 0)]
    [InlineData(AppTheme.Light, 1)]
    [InlineData(AppTheme.Dark, 2)]
    [InlineData("nope", 0)]
    public void Index_round_trips_selector_choices(string value, int index)
    {
        Assert.Equal(index, AppTheme.IndexOf(value));
        Assert.Equal(AppTheme.Normalize(value), AppTheme.FromIndex(index));
    }

    [Fact]
    public void Missing_theme_in_settings_defaults_to_system()
    {
        var path = Path.Combine(Path.GetTempPath(), "mtgo-theme-" + Guid.NewGuid().ToString("N") + ".json");
        try
        {
            File.WriteAllText(path, """
                { "schemaVersion": 1, "overlayEnabled": true }
                """);
            var store = new SettingsStore(path);
            Assert.True(store.Load().IsSuccess);
            Assert.Equal(AppTheme.System, store.Current.Theme);
        }
        finally
        {
            File.Delete(path);
        }
    }
}
