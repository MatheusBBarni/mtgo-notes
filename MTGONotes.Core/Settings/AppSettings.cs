using System.Text.Json;
using MTGONotes.Core.Disclosure;

namespace MTGONotes.Core.Settings;

public sealed class AppSettings
{
    public int SchemaVersion { get; set; } = 1;

    public bool LiveAttachAcknowledged { get; set; }

    public bool LiveAttachEnabled { get; set; } = true;

    public bool OfficialDeckConsent { get; set; }

    public bool OverlayEnabled { get; set; } = true;

    public int OverlayX { get; set; } = 16;

    public int OverlayY { get; set; } = 16;

    public bool OverlayMinimized { get; set; }

    public bool TrayEnabled { get; set; } = true;

    public bool LaunchWithWindows { get; set; }

    public bool DiagnosticsEnabled { get; set; }

    public string Theme { get; set; } = AppTheme.System;
}

public sealed class SettingsStore
{
    private static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
    };

    private readonly string _path;

    public SettingsStore(string path) => _path = path;

    public AppSettings Current { get; private set; } = new();

    public Result Load()
    {
        if (!File.Exists(_path))
        {
            Current = new AppSettings();
            return Result.Ok();
        }

        try
        {
            var json = File.ReadAllText(_path);
            Current = JsonSerializer.Deserialize<AppSettings>(json, Json) ?? new AppSettings();
            return Result.Ok();
        }
        catch (JsonException)
        {
            return Result.Fail(Domain.RepoError.InvalidRequest);
        }
    }

    public Result Save(AppSettings settings)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(_path)!);
            var partial = _path + ".partial";
            File.WriteAllText(partial, JsonSerializer.Serialize(settings, Json));
            File.Move(partial, _path, overwrite: true);
            Current = settings;
            return Result.Ok();
        }
        catch (IOException)
        {
            return Result.Fail(Domain.RepoError.DestinationUnwritable);
        }
    }
}
