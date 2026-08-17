using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;
using MTGONotes.Core.Live;

namespace MTGONotes.Core.Facades;

public interface IOverlayFacade
{
    OverlayView CurrentView { get; }

    OpponentCandidate? Candidate { get; }

    Result ConfirmOpponent(OpponentCandidate candidate);

    Result EnterOpponent(string handle);

    Result CorrectPhase(InternalPhase phase);

    Result OpenCapture();

    Result FinishEncounter();

    Result UndoTransition();

    Result PauseDetection(bool paused);
}

public interface ICaptureFacade
{
    Result SaveObservation(string text);

    Result DiscardDraft();
}

public interface INotebookFacade
{
    Result AuthorizeHistory();

    OverlayView CurrentView { get; }
}
