namespace MTGONotes.Core.Encounters;

public static class EncounterSource
{
    public static string Normalize(string source)
    {
        if (source is "uia" or "ocr" or "mtgosdk" or "manual")
        {
            return source;
        }

        return source.StartsWith("mtgosdk", StringComparison.Ordinal) ? "mtgosdk" : "manual";
    }
}
