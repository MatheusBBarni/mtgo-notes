using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Encounters;
using MTGONotes.Core.Facades;
using MTGONotes.Core.Identity;
using MTGONotes.Core.Live;
using MTGONotes.Core.Notebook;

namespace MTGONotes.Core.Session;

public sealed class CompanionSession : IOverlayFacade, ICaptureFacade, INotebookFacade
{
    private readonly EncounterReducer _reducer = new();
    private readonly DisclosurePolicy _disclosure = new();
    private readonly INotebookStore? _store;
    private readonly List<ObservationView> _currentNotes = [];
    private readonly object _gate = new();

    public CompanionSession(INotebookStore? store = null)
    {
        _store = store;
    }
    private EncounterRuntime _runtime = EncounterRuntime.Idle("unattached");
    private bool _paused;
    private bool _captureOpen;
    private string? _confirmedDisplay;
    private string? _confirmedKey;

    public event EventHandler<OverlayView>? OverlayChanged;

    public OverlayView CurrentView { get; private set; } = DisclosurePolicy.Neutral;

    public OpponentCandidate? Candidate { get; private set; }

    public EntityId? ActiveEncounterId
    {
        get
        {
            lock (_gate)
            {
                return _runtime.Active?.Id;
            }
        }
    }

    public bool DetectionPaused
    {
        get
        {
            lock (_gate)
            {
                return _paused;
            }
        }
    }

    public void ApplySnapshot(LiveSnapshot snapshot)
    {
        lock (_gate)
        {
            if (_paused)
            {
                return;
            }

            if (snapshot.Opponent is { } opponent
                && !string.Equals(opponent.NormalizedHandle, _confirmedKey, StringComparison.Ordinal))
            {
                Candidate = opponent;
            }

            if (_runtime.Active is not null
                && snapshot.SuggestedPhase != InternalPhase.Idle
                && snapshot.SuggestedPhase != _runtime.Phase)
            {
                var evidence = new ContextEvidence(
                    _runtime.ProviderSession,
                    _runtime.Generation,
                    Math.Max(_runtime.LastSequence + 1, snapshot.Sequence),
                    snapshot.MonotonicMs,
                    EvidenceSource.Mtgosdk,
                    snapshot.SuggestedPhase == InternalPhase.InGameRestricted
                        ? new EvidenceKind.StrongGameplay()
                        : new EvidenceKind.TrustedPhase(snapshot.SuggestedPhase, EncounterReducer.OcrStableDurationMs));
                ApplyReduction(_reducer.Reduce(_runtime, evidence));
            }

            Publish();
        }
    }

    public Result ConfirmOpponent(OpponentCandidate candidate)
    {
        lock (_gate)
        {
            return ConfirmUnlocked(candidate, requireListedCandidate: true);
        }
    }

    public Result EnterOpponent(string handle)
    {
        var normalized = HandleNormalization.NormalizeHandle(handle);
        if (!normalized.IsSuccess)
        {
            return Result.Fail(normalized.Error!.Value);
        }

        lock (_gate)
        {
            var candidate = new OpponentCandidate(
                normalized.Value!.Display,
                normalized.Value.Key,
                null,
                1,
                1,
                "manual");
            Candidate = candidate;
            return ConfirmUnlocked(candidate, requireListedCandidate: false);
        }
    }

    private Result ConfirmUnlocked(OpponentCandidate candidate, bool requireListedCandidate)
    {
        if (requireListedCandidate
            && (Candidate is null
                || Candidate.Generation != candidate.Generation
                || Candidate.Sequence != candidate.Sequence
                || Candidate.ProviderSession != candidate.ProviderSession))
        {
            return Result.Fail(RepoError.CandidateStale);
        }

        var normalized = HandleNormalization.NormalizeHandle(candidate.DisplayHandle);
        if (!normalized.IsSuccess)
        {
            return Result.Fail(normalized.Error!.Value);
        }

        var session = string.IsNullOrWhiteSpace(candidate.ProviderSession)
            ? "manual"
            : candidate.ProviderSession;
        if (_runtime.ProviderSession != session)
        {
            _runtime = EncounterRuntime.Idle(session);
        }

        var profileId = EntityId.New();
        if (_store is not null)
        {
            var existing = _store.FindProfileByNormalizedHandle(normalized.Value!.Key);
            if (!existing.IsSuccess)
            {
                return Result.Fail(existing.Error!.Value);
            }

            if (existing.Value is { } found)
            {
                profileId = found;
            }
        }

        var result = _reducer.Reduce(
            _runtime,
            new ContextEvidence(
                session,
                Math.Max(_runtime.Generation + 1, candidate.Generation),
                Math.Max(1, candidate.Sequence),
                0,
                session == "manual" ? EvidenceSource.Manual : EvidenceSource.Mtgosdk,
                new EvidenceKind.ConfirmedOpponent(profileId, EntityId.New())));
        if (!result.IsSuccess)
        {
            return Result.Fail(result.Error!.Value);
        }

        var persisted = Persist(
            result.Value!.Actions,
            session,
            normalized.Value!.Display,
            normalized.Value.Key,
            result.Value.Runtime.Generation);
        if (!persisted.IsSuccess)
        {
            return persisted;
        }

        ApplyReduction(result);
        _confirmedDisplay = normalized.Value.Display;
        _confirmedKey = normalized.Value.Key;
        Candidate = null;
        if (result.Value.Actions.OfType<EncounterAction.StartEncounter>().Any())
        {
            _currentNotes.Clear();
        }

        Publish();
        return Result.Ok();
    }

    public Result CorrectPhase(InternalPhase phase)
    {
        lock (_gate)
        {
            var result = _reducer.Reduce(
                _runtime,
                new ContextEvidence(
                    _runtime.ProviderSession,
                    _runtime.Generation,
                    _runtime.LastSequence + 1,
                    0,
                    EvidenceSource.Manual,
                    new EvidenceKind.TrustedPhase(phase, EncounterReducer.OcrStableDurationMs)));
            if (!result.IsSuccess)
            {
                return Result.Fail(result.Error!.Value);
            }

            var persisted = Persist(
                result.Value!.Actions,
                _runtime.ProviderSession,
                null,
                null,
                result.Value.Runtime.Generation);
            if (!persisted.IsSuccess)
            {
                return persisted;
            }

            ApplyReduction(result);
            Publish();
            return Result.Ok();
        }
    }

    public Result OpenCapture()
    {
        lock (_gate)
        {
            if (_runtime.Active is null)
            {
                return Result.Fail(RepoError.NoActiveEncounter);
            }

            if (_captureOpen)
            {
                return Result.Fail(RepoError.AlreadyOpen);
            }

            _captureOpen = true;
            return Result.Ok();
        }
    }

    public Result FinishEncounter()
    {
        lock (_gate)
        {
            var result = _reducer.Reduce(
                _runtime,
                new ContextEvidence(
                    _runtime.ProviderSession,
                    _runtime.Generation,
                    _runtime.LastSequence + 1,
                    0,
                    EvidenceSource.Manual,
                    new EvidenceKind.End()));
            if (!result.IsSuccess)
            {
                return Result.Fail(result.Error!.Value);
            }

            var persisted = Persist(
                result.Value!.Actions,
                _runtime.ProviderSession,
                null,
                null,
                result.Value.Runtime.Generation);
            if (!persisted.IsSuccess)
            {
                return persisted;
            }

            ApplyReduction(result);
            Publish();
            return Result.Ok();
        }
    }

    public Result UndoTransition() => Result.Fail(RepoError.UndoExpired);

    public Result PauseDetection(bool paused)
    {
        lock (_gate)
        {
            _paused = paused;
            if (paused)
            {
                Candidate = null;
                if (_runtime.Active is not null && !_runtime.Phase.IsDisclosureRestricted())
                {
                    ApplyReduction(
                        _reducer.Reduce(
                            _runtime,
                            new ContextEvidence(
                                _runtime.ProviderSession,
                                _runtime.Generation,
                                _runtime.LastSequence + 1,
                                0,
                                EvidenceSource.System,
                                new EvidenceKind.StrongGameplay())));
                }
            }

            Publish();
            return Result.Ok();
        }
    }

    public Result SaveObservation(string text)
    {
        lock (_gate)
        {
            if (_runtime.Active is null)
            {
                return Result.Fail(RepoError.NoActiveEncounter);
            }

            if (string.IsNullOrWhiteSpace(text))
            {
                return Result.Fail(RepoError.BlankObservation);
            }

            var observationId = EntityId.New();
            var trimmed = text.Trim();
            if (_store is not null)
            {
                var saved = _store.SaveObservation(
                    observationId,
                    _runtime.Active.Id,
                    trimmed,
                    UtcMillis.Now());
                if (!saved.IsSuccess)
                {
                    return saved;
                }
            }

            _currentNotes.Insert(0, new ObservationView(observationId.AsString(), trimmed, false));
            _captureOpen = false;
            Publish();
            return Result.Ok();
        }
    }

    public Result DiscardDraft()
    {
        lock (_gate)
        {
            _captureOpen = false;
            return Result.Ok();
        }
    }

    public Result AuthorizeHistory() => _disclosure.Authorize(QueryKind.SearchHistory, CurrentView.Phase);

    private Result Persist(
        IReadOnlyList<EncounterAction> actions,
        string source,
        string? displayHandle,
        string? normalizedHandle,
        ulong generation)
    {
        if (_store is null)
        {
            return Result.Ok();
        }

        var now = UtcMillis.Now();
        foreach (var action in actions)
        {
            var persisted = action switch
            {
                EncounterAction.ResolveProfile resolve when displayHandle is not null && normalizedHandle is not null =>
                    PersistProfile(resolve.ProfileId, displayHandle, normalizedHandle, now),
                EncounterAction.StartEncounter start => _store.StartEncounter(
                    start.EncounterId,
                    start.ProfileId,
                    now,
                    generation,
                    source is "uia" or "ocr" or "mtgosdk" ? source : "manual"),
                EncounterAction.FinishEncounter finish => _store.FinishEncounter(finish.EncounterId, now),
                EncounterAction.ChangePhase change => _store.ChangePhase(change.EncounterId, change.To),
                EncounterAction.MarkIncomplete incomplete => _store.MarkIncomplete(
                    incomplete.EncounterId,
                    "completion_ignored"),
                _ => Result.Ok(),
            };
            if (!persisted.IsSuccess)
            {
                return persisted;
            }
        }

        return Result.Ok();
    }

    private Result PersistProfile(
        EntityId profileId,
        string displayHandle,
        string normalizedHandle,
        UtcMillis createdAt)
    {
        var existing = _store!.FindProfileByNormalizedHandle(normalizedHandle);
        if (!existing.IsSuccess)
        {
            return Result.Fail(existing.Error!.Value);
        }

        return existing.Value is null
            ? _store.CreateProfile(profileId, displayHandle, normalizedHandle, createdAt)
            : Result.Ok();
    }

    private void ApplyReduction(Result<Reduction> result)
    {
        if (!result.IsSuccess)
        {
            return;
        }

        _runtime = result.Value!.Runtime;
    }

    private void Publish()
    {
        CurrentView = _disclosure.Overlay(
            new NotebookState(
                _runtime.Phase,
                _confirmedDisplay,
                false,
                _currentNotes.ToArray(),
                [],
                null));
        OverlayChanged?.Invoke(this, CurrentView);
    }
}
