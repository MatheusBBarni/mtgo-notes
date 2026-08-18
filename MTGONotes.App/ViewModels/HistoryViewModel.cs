using System.Collections.ObjectModel;
using MTGONotes.App.Host;
using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Portability;

namespace MTGONotes.App.ViewModels;

public sealed class HistoryViewModel : ViewModel
{
    private readonly AppHost _host;
    private bool _restricted;
    private string _query = string.Empty;
    private string _statusMessage = string.Empty;
    private bool _isStatusOpen;
    private bool _isStatusError;
    private bool _isRestricted;
    private bool _isEmpty;
    private string _emptyTitle = "No encounters yet";
    private string _emptyDetail = "Confirmed opponents appear here after an encounter is recorded.";

    public HistoryViewModel(AppHost host) => _host = host;

    public string Query
    {
        get => _query;
        set
        {
            if (SetProperty(ref _query, value))
            {
                Refresh();
            }
        }
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

    public bool IsRestricted
    {
        get => _isRestricted;
        set => SetProperty(ref _isRestricted, value);
    }

    public bool IsEmpty
    {
        get => _isEmpty;
        set => SetProperty(ref _isEmpty, value);
    }

    public string EmptyTitle
    {
        get => _emptyTitle;
        set => SetProperty(ref _emptyTitle, value);
    }

    public string EmptyDetail
    {
        get => _emptyDetail;
        set => SetProperty(ref _emptyDetail, value);
    }

    public ObservableCollection<HistoryRow> Items { get; } = [];

    public void NotifyDisclosure(InternalPhase phase)
    {
        var restricted = phase.IsDisclosureRestricted();
        if (restricted == _restricted)
        {
            return;
        }

        Refresh();
    }

    public void Refresh()
    {
        if (string.IsNullOrWhiteSpace(Query))
        {
            ListRecent();
            return;
        }

        Search();
    }

    public async Task ExportToAsync(string path)
    {
        var dump = _host.Session.ExportLogical();
        if (!dump.IsSuccess)
        {
            SetStatus(dump.Error!.Value.ToAppError().Message, isError: true);
            return;
        }

        await File.WriteAllTextAsync(path, TextExporter.Render(dump.Value!));
        SetStatus("Export written.", isError: false);
    }

    public void WriteBackup(string path, string passphrase)
    {
        if (!_host.Operations.Begin("backup").IsSuccess)
        {
            SetStatus("Another notebook operation is running.", isError: true);
            return;
        }

        try
        {
            var dump = _host.Session.ExportLogical();
            if (!dump.IsSuccess)
            {
                SetStatus(dump.Error!.Value.ToAppError().Message, isError: true);
                return;
            }

            var written = NotebookBackup.Write(path, dump.Value!, passphrase);
            SetStatus(
                written.IsSuccess ? "Backup written." : written.Error!.Value.ToAppError().Message,
                isError: !written.IsSuccess);
        }
        finally
        {
            _host.Operations.End();
        }
    }

    private void ListRecent()
    {
        var result = _host.Session.ListRecentEncounters();
        if (!result.IsSuccess)
        {
            ShowFailure(result.Error!.Value);
            return;
        }

        Items.Clear();
        foreach (var item in result.Value!)
        {
            Items.Add(
                new HistoryRow(
                    item.Handle,
                    $"{item.Status} · {item.Phase} · {PresentationText.FormatTimestamp(item.StartedAt)}"));
        }

        ShowCollection("No encounters yet", "Confirmed opponents appear here after an encounter is recorded.");
    }

    private void Search()
    {
        var result = _host.Session.SearchHistory(Query);
        if (!result.IsSuccess)
        {
            ShowFailure(result.Error!.Value);
            return;
        }

        Items.Clear();
        foreach (var item in result.Value!.Items)
        {
            Items.Add(new HistoryRow(item.EntityType, item.Content));
        }

        ShowCollection("No matching history", "Try a different search, or clear the box to list recent encounters.");
    }

    private void ShowFailure(RepoError error)
    {
        var appError = error.ToAppError();
        _restricted = error == RepoError.DisclosureRestricted;
        IsRestricted = _restricted;
        Items.Clear();
        IsEmpty = true;
        EmptyTitle = _restricted ? "History hidden" : "History unavailable";
        EmptyDetail = appError.Message;
        SetStatus(appError.Message, isError: true);
    }

    private void ShowCollection(string emptyTitle, string emptyDetail)
    {
        _restricted = false;
        IsRestricted = false;
        IsEmpty = Items.Count == 0;
        EmptyTitle = emptyTitle;
        EmptyDetail = emptyDetail;
        if (!IsEmpty)
        {
            IsStatusOpen = false;
        }
    }

    private void SetStatus(string message, bool isError)
    {
        StatusMessage = message;
        IsStatusError = isError;
        IsStatusOpen = !string.IsNullOrWhiteSpace(message);
    }
}
