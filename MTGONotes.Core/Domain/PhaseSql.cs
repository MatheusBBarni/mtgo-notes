namespace MTGONotes.Core.Domain;

public static class PhaseSql
{
    public static string ToSql(this InternalPhase phase) =>
        phase switch
        {
            InternalPhase.Idle => "idle",
            InternalPhase.Candidate => "candidate",
            InternalPhase.PreMatch => "pre_match",
            InternalPhase.InGameRestricted => "in_game_restricted",
            InternalPhase.BetweenGames => "between_games",
            InternalPhase.CompletionPending => "completion_pending",
            InternalPhase.Finished => "finished",
            InternalPhase.Incomplete => "incomplete",
            _ => "in_game_restricted",
        };

    public static InternalPhase FromSql(string value) =>
        value switch
        {
            "idle" => InternalPhase.Idle,
            "candidate" => InternalPhase.Candidate,
            "pre_match" => InternalPhase.PreMatch,
            "in_game_restricted" => InternalPhase.InGameRestricted,
            "between_games" => InternalPhase.BetweenGames,
            "completion_pending" => InternalPhase.CompletionPending,
            "finished" => InternalPhase.Finished,
            "incomplete" => InternalPhase.Incomplete,
            _ => InternalPhase.InGameRestricted,
        };
}
