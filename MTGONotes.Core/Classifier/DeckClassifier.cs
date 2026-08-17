using MTGONotes.Core.Disclosure;
using MTGONotes.Core.Domain;

namespace MTGONotes.Core.Classifier;

public enum ClassificationMethod
{
    Signature,
    Knn,
    Unsupported,
}

public sealed record CanonicalCard(string OracleId, int Quantity, bool BasicLand = false);

public sealed record CompleteDeck(string Format, bool Complete, IReadOnlyList<CanonicalCard> Cards);

public sealed record SignatureConstraint(
    string OracleId,
    string DisplayName,
    int? MinCopies,
    int? ExactCopies);

public sealed record ArchetypeDefinition(
    string Id,
    string DisplayName,
    bool StrictMode,
    IReadOnlyList<SignatureConstraint> Signatures);

public sealed record FormatDefinition(
    string Name,
    int K,
    double MinConfidence,
    IReadOnlyList<ArchetypeDefinition> Archetypes);

public sealed record CorpusDeck(
    string Id,
    string Format,
    string ArchetypeId,
    IReadOnlyDictionary<string, int> Cards);

public sealed record ClassifierAssets(
    string ClassifierVersion,
    string Digest,
    IReadOnlyList<FormatDefinition> Formats,
    IReadOnlyList<CorpusDeck> Corpus);

public sealed record NeighborExplanation(string CorpusId, string ArchetypeId, double Similarity);

public sealed record ClassificationExplanation(
    string Summary,
    IReadOnlyList<string> MatchedSignatureCards,
    IReadOnlyList<NeighborExplanation> Neighbors,
    string? DecisiveRule);

public sealed record ClassificationResult(
    string ResultId,
    string ResultName,
    string ClassifierVersion,
    string ClassifierDigest,
    ClassificationMethod Method,
    double Confidence,
    ClassificationExplanation Explanation);

public static class DeckClassifier
{
    public static Result<ClassificationResult> Classify(CompleteDeck deck, ClassifierAssets assets)
    {
        if (!deck.Complete)
        {
            return Result<ClassificationResult>.Fail(RepoError.DeckIncomplete);
        }

        var format = assets.Formats.FirstOrDefault(item =>
            item.Name.Equals(deck.Format, StringComparison.OrdinalIgnoreCase));
        if (format is null)
        {
            return Result<ClassificationResult>.Fail(RepoError.FormatUnsupported);
        }

        var vector = CanonicalVector(deck);
        var matching = format.Archetypes
            .Select((archetype, order) => (Specificity: Specificity(archetype), Order: order, Archetype: archetype))
            .Where(item => SignatureMatches(item.Archetype, vector))
            .OrderByDescending(item => item.Specificity)
            .ThenBy(item => item.Order)
            .ToArray();
        if (matching.Length > 0)
        {
            var archetype = matching[0].Archetype;
            return Result<ClassificationResult>.Ok(
                new ClassificationResult(
                    archetype.Id,
                    archetype.DisplayName,
                    assets.ClassifierVersion,
                    assets.Digest,
                    ClassificationMethod.Signature,
                    1.0,
                    new ClassificationExplanation(
                        $"Matched all {archetype.Signatures.Count} signature constraints for {archetype.DisplayName}.",
                        archetype.Signatures.Select(item => item.DisplayName).ToArray(),
                        [],
                        archetype.Id)));
        }

        var strict = format.Archetypes.Where(item => item.StrictMode).Select(item => item.Id).ToHashSet();
        var neighbors = assets.Corpus
            .Where(entry => entry.Format.Equals(format.Name, StringComparison.OrdinalIgnoreCase))
            .Where(entry => !strict.Contains(entry.ArchetypeId))
            .Select(entry => new NeighborExplanation(
                entry.Id,
                entry.ArchetypeId,
                Cosine(vector, entry.Cards)))
            .OrderByDescending(item => item.Similarity)
            .ThenBy(item => item.CorpusId, StringComparer.Ordinal)
            .Take(format.K)
            .ToArray();
        var weights = new Dictionary<string, double>(StringComparer.Ordinal);
        var total = 0.0;
        foreach (var neighbor in neighbors)
        {
            weights[neighbor.ArchetypeId] = weights.GetValueOrDefault(neighbor.ArchetypeId) + neighbor.Similarity;
            total += neighbor.Similarity;
        }

        var order = format.Archetypes.Select((item, index) => (item.Id, index)).ToDictionary(item => item.Id, item => item.index);
        var winner = weights
            .OrderByDescending(item => item.Value)
            .ThenByDescending(item => order.GetValueOrDefault(item.Key, int.MaxValue))
            .FirstOrDefault();
        var confidence = total > 0 && winner.Key is not null ? winner.Value / total : 0.0;
        var accepted = winner.Key is not null && confidence >= format.MinConfidence;
        var definition = accepted
            ? format.Archetypes.FirstOrDefault(item => item.Id == winner.Key)
            : null;
        return Result<ClassificationResult>.Ok(
            new ClassificationResult(
                definition?.Id ?? "unclassified",
                definition?.DisplayName ?? "Unclassified",
                assets.ClassifierVersion,
                assets.Digest,
                ClassificationMethod.Knn,
                confidence,
                new ClassificationExplanation(
                    accepted
                        ? $"Top {neighbors.Length} local neighbors produced {confidence:0.000} confidence."
                        : $"Local neighbor confidence {confidence:0.000} is below the {format.MinConfidence:0.000} threshold.",
                    [],
                    neighbors,
                    winner.Key)));
    }

    public static IReadOnlyDictionary<string, int> CanonicalVector(CompleteDeck deck)
    {
        var counts = new Dictionary<string, (int Quantity, bool Basic)>(StringComparer.Ordinal);
        foreach (var card in deck.Cards)
        {
            if (counts.TryGetValue(card.OracleId, out var current))
            {
                counts[card.OracleId] = (current.Quantity + card.Quantity, current.Basic || card.BasicLand);
            }
            else
            {
                counts[card.OracleId] = (card.Quantity, card.BasicLand);
            }
        }

        return counts.ToDictionary(
            item => item.Key,
            item => item.Value.Basic ? item.Value.Quantity : Math.Min(item.Value.Quantity, 4),
            StringComparer.Ordinal);
    }

    private static bool SignatureMatches(
        ArchetypeDefinition archetype,
        IReadOnlyDictionary<string, int> vector)
    {
        return archetype.Signatures.Count > 0
            && archetype.Signatures.All(constraint =>
            {
                var copies = vector.GetValueOrDefault(constraint.OracleId);
                return constraint.ExactCopies is { } exact
                    ? copies == exact
                    : copies >= (constraint.MinCopies ?? 1);
            });
    }

    private static int Specificity(ArchetypeDefinition archetype) =>
        (archetype.Signatures.Count * 2) + archetype.Signatures.Count(item => item.ExactCopies is not null);

    private static double Cosine(
        IReadOnlyDictionary<string, int> left,
        IReadOnlyDictionary<string, int> right)
    {
        var dot = left.Sum(item => item.Value * right.GetValueOrDefault(item.Key));
        var leftNorm = Math.Sqrt(left.Values.Sum(value => value * value));
        var rightNorm = Math.Sqrt(right.Values.Sum(value => value * value));
        return leftNorm == 0 || rightNorm == 0 ? 0 : dot / (leftNorm * rightNorm);
    }
}
