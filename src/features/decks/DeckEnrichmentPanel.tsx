import type {
  DeckCandidate,
  DeckDetails,
  InteractiveLookup,
} from "../../lib/ipc/decks";
import { openOfficialDeckPage } from "../../lib/ipc/decks";
import { Button, Panel, StatusLabel } from "../../ui/primitives";
import { ClassificationResult } from "../classifier/ClassificationResult";

export type DeckEnrichmentPanelProps = {
  lookup?: InteractiveLookup;
  candidate?: DeckCandidate;
  deck?: DeckDetails;
  onOpenOfficial?: (url: string) => void;
  onConfirm?: () => void;
};

export function DeckEnrichmentPanel({
  lookup,
  candidate,
  deck,
  onOpenOfficial,
  onConfirm,
}: DeckEnrichmentPanelProps) {
  const openOfficial = (url: string) => {
    if (onOpenOfficial) {
      onOpenOfficial(url);
      return;
    }
    void openOfficialDeckPage(url);
  };

  return (
    <Panel label="Official deck enrichment">
      <section aria-labelledby="official-deck-title">
        <div className="section-heading">
          <div>
            <h2 id="official-deck-title">Official deck context</h2>
            <p className="muted-copy">
              Automatic access is disabled. Only a user-confirmed official MTGO
              result can be saved.
            </p>
          </div>
          <StatusLabel kind="source" label="Official MTGO · interactive" />
        </div>

        {lookup ? (
          <div role="status" className="provider-confirmation">
            <p>
              Review the official page for {lookup.binding.confirmedHandle} in{" "}
              {lookup.binding.format}. The app does not scrape the page.
            </p>
            <Button
              variant="secondary"
              onClick={() => openOfficial(lookup.officialUrl)}
            >
              Open official MTGO page
            </Button>
          </div>
        ) : (
          <p>
            Provider consent discloses only the confirmed opponent handle and
            format. Local notebook work remains available without enrichment.
          </p>
        )}

        {candidate ? (
          <article className="deck-provenance">
            <h3>{candidate.providerLabel ?? "Official deck candidate"}</h3>
            <p>
              {candidate.event} · {candidate.format} ·{" "}
              {new Date(candidate.publicationDate).toLocaleDateString()}
            </p>
            <p>
              {candidate.cards.reduce(
                (total, card) => total + card.quantity,
                0,
              )}{" "}
              cards from {candidate.provider}. Review its official source before
              confirmation.
            </p>
            <Button
              variant="secondary"
              onClick={() => openOfficial(candidate.sourceUrl)}
            >
              Review official source
            </Button>{" "}
            <Button onClick={onConfirm}>Confirm official result</Button>
          </article>
        ) : null}

        {deck?.publicSnapshot ? (
          <article className="deck-provenance">
            <h3>{deck.providerLabel ?? "Confirmed official deck"}</h3>
            <dl className="provenance-grid">
              <div>
                <dt>Event</dt>
                <dd>{deck.publicSnapshot.event}</dd>
              </div>
              <div>
                <dt>Format</dt>
                <dd>{deck.publicSnapshot.format}</dd>
              </div>
              <div>
                <dt>Published</dt>
                <dd>
                  {new Date(
                    deck.publicSnapshot.publicationDate,
                  ).toLocaleDateString()}
                </dd>
              </div>
              <div>
                <dt>Source</dt>
                <dd>{deck.publicSnapshot.provider}</dd>
              </div>
            </dl>
            <p>
              Complete deck revision {deck.revisionNumber} · {deck.cards.length}{" "}
              distinct entries
            </p>
          </article>
        ) : null}

        {deck ? (
          <ClassificationResult
            result={deck.currentClassification}
            previousRuns={deck.classificationHistory.slice(1)}
          />
        ) : null}
      </section>
    </Panel>
  );
}
