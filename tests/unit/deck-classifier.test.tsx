import { render, screen } from "@testing-library/react";

import { ClassificationResult } from "../../src/features/classifier/ClassificationResult";
import { DeckEnrichmentPanel } from "../../src/features/decks/DeckEnrichmentPanel";
import type { ClassificationRun, DeckDetails } from "../../src/lib/ipc/decks";

const classification: ClassificationRun = {
  id: "run-1",
  deckRevisionId: "revision-1",
  classifierVersion: "2026.07.1",
  classifierDigest: "sha256:fixture",
  resultId: "burn",
  resultName: "Burn",
  method: "knn",
  confidence: 0.75,
  explanation: {
    summary: "Top five local neighbors produced 0.750 confidence.",
    matchedSignatureCards: [],
    neighbors: [
      { corpusId: "burn-001", archetypeId: "burn", similarity: 0.95 },
    ],
  },
  status: "successful",
  createdAt: 1_753_689_600_000,
};

describe("read-only classifier and official deck surfaces", () => {
  test("UT-111: result shows method, version, confidence, and explanation without editor controls", () => {
    render(<ClassificationResult result={classification} />);

    expect(screen.getByRole("heading", { name: "Burn" })).toBeInTheDocument();
    expect(screen.getByText("2026.07.1")).toBeInTheDocument();
    expect(screen.getByText("75%")).toBeInTheDocument();
    expect(screen.getByText("Local nearest neighbors")).toBeInTheDocument();
    expect(
      screen.getByText("Top five local neighbors produced 0.750 confidence."),
    ).toBeInTheDocument();

    for (const name of [
      /edit archetype/i,
      /import/i,
      /activate/i,
      /delete definition/i,
      /save definition/i,
    ]) {
      expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
    }
  });

  test("IT-248 and provenance UI: official and local labels remain visibly separate", () => {
    const deck: DeckDetails = {
      deckId: "deck-1",
      deckRevisionId: "revision-1",
      revisionNumber: 1,
      canonicalDigest: "digest",
      complete: true,
      format: "Modern",
      sourceClass: "public",
      providerLabel: "Provider Burn",
      cards: [
        {
          oracleId: "oracle-lightning-bolt",
          displayName: "Lightning Bolt",
          zone: "main",
          quantity: 4,
        },
      ],
      publicSnapshot: {
        id: "snapshot-1",
        encounterId: "encounter-1",
        provider: "official_mtgo",
        event: "Fixture Challenge",
        format: "Modern",
        publicationDate: 1_753_689_600_000,
        sourceUrl: "https://www.mtgo.com/decklists/fixture",
        confirmed: true,
        sourceToken: "token-1",
        createdAt: 1_753_689_600_000,
      },
      currentClassification: classification,
      classificationHistory: [classification],
    };

    render(<DeckEnrichmentPanel deck={deck} />);

    expect(
      screen.getByRole("heading", { name: "Provider Burn" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Burn" })).toBeInTheDocument();
    expect(screen.getByText("official_mtgo")).toBeInTheDocument();
    expect(
      screen.getByText(/Automatic access is disabled/),
    ).toBeInTheDocument();
  });
});
