using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Notebook;

public interface INotebookStore
{
    Result<EntityId?> FindProfileByNormalizedHandle(string normalizedHandle);

    Result CreateProfile(EntityId id, string displayHandle, string normalizedHandle, UtcMillis createdAt);

    Result StartEncounter(
        EntityId encounterId,
        EntityId profileId,
        UtcMillis startedAt,
        ulong generation,
        string source);

    Result FinishEncounter(EntityId encounterId, UtcMillis endedAt);

    Result ChangePhase(EntityId encounterId, InternalPhase phase);

    Result MarkIncomplete(EntityId encounterId, string reason);

    Result SaveObservation(
        EntityId observationId,
        EntityId encounterId,
        string text,
        UtcMillis createdAt);

    Result<IReadOnlyList<ObservationView>> ListObservations(EntityId encounterId);
}
