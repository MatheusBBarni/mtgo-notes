using MTGONotes.Core.Identity;
using MTGONotes.Core.Live;

namespace MTGONotes.Core.Detection;

public sealed record VisibleWindow(string Title, ulong Handle);

public interface IWindowEnumerator
{
    IReadOnlyList<VisibleWindow> ListVisible();
}

public sealed class TitleFallbackSource : IContextSource
{
    public const string SourceId = "title_fallback";

    private readonly IWindowEnumerator _windows;

    public TitleFallbackSource(IWindowEnumerator windows) => _windows = windows;

    public string Id => SourceId;

    public event EventHandler<LiveSnapshot>? SnapshotChanged;

    public Task StartAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    public Task StopAsync() => Task.CompletedTask;

    public void PollOnce()
    {
        var mtgo = _windows.ListVisible()
            .FirstOrDefault(window =>
                window.Title.Contains("Magic: The Gathering Online", StringComparison.OrdinalIgnoreCase)
                || window.Title.Contains("MTGO", StringComparison.OrdinalIgnoreCase));
        if (mtgo is null)
        {
            return;
        }

        SnapshotChanged?.Invoke(
            this,
            new LiveSnapshot(
                "title-fallback",
                1,
                1,
                0,
                null,
                Domain.InternalPhase.InGameRestricted,
                null,
                null,
                null));
    }
}
