using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Notebook;

public sealed record HistoryHit(string EntityType, string EntityId, long SortMs, string Content);

public sealed record HistoryPage(IReadOnlyList<HistoryHit> Items, bool HasMore);

public sealed record EncounterSummary(
    EntityId Id,
    EntityId ProfileId,
    string Handle,
    string Phase,
    string Status,
    long StartedAt);

public sealed record LogicalObservation(string Id, string EncounterId, string Text, long CreatedAt);

public sealed record LogicalEncounter(
    string Id,
    string ProfileId,
    string Handle,
    string Phase,
    string Status,
    long StartedAt,
    IReadOnlyList<LogicalObservation> Observations);

public sealed record LogicalProfile(string Id, string Handle, string NormalizedHandle, long CreatedAt);

public sealed record NotebookDump(
    int SchemaVersion,
    IReadOnlyList<LogicalProfile> Profiles,
    IReadOnlyList<LogicalEncounter> Encounters);
