namespace MTGONotes.Core.Settings;

public static class AppTheme
{
    public const string System = "system";
    public const string Light = "light";
    public const string Dark = "dark";

    public static string Normalize(string? value) =>
        value is Light or Dark ? value : System;

    public static int IndexOf(string? value) =>
        Normalize(value) switch
        {
            Light => 1,
            Dark => 2,
            _ => 0,
        };

    public static string FromIndex(int index) =>
        index switch
        {
            1 => Light,
            2 => Dark,
            _ => System,
        };
}
