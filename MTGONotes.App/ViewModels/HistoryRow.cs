namespace MTGONotes.App.ViewModels;

public sealed record HistoryRow(string Title, string Detail)
{
    public bool HasDetail => !string.IsNullOrWhiteSpace(Detail);
}
