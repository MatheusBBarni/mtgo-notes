import type { ClassificationRun } from "../../lib/ipc/decks";
import { StatusLabel } from "../../ui/primitives";

export type ClassificationResultProps = {
  result?: ClassificationRun;
  previousRuns?: ClassificationRun[];
};

export function ClassificationResult({
  result,
  previousRuns = [],
}: ClassificationResultProps) {
  if (!result) {
    return (
      <section aria-labelledby="classification-title">
        <h3 id="classification-title">Local archetype</h3>
        <p>No completed local classification is available.</p>
      </section>
    );
  }

  return (
    <section aria-labelledby="classification-title">
      <div className="section-heading">
        <div>
          <h3 id="classification-title">{result.resultName}</h3>
          <p className="muted-copy">
            Derived locally from this complete deck revision.
          </p>
        </div>
        <StatusLabel kind="source" label={result.method} />
      </div>
      <dl className="provenance-grid">
        <div>
          <dt>Classifier</dt>
          <dd>{result.classifierVersion}</dd>
        </div>
        <div>
          <dt>Confidence</dt>
          <dd>{Math.round(result.confidence * 100)}%</dd>
        </div>
        <div>
          <dt>Method</dt>
          <dd>
            {result.method === "knn"
              ? "Local nearest neighbors"
              : result.method}
          </dd>
        </div>
      </dl>
      <p>{result.explanation.summary}</p>
      {result.explanation.matchedSignatureCards.length > 0 ? (
        <p>
          Signature cards: {result.explanation.matchedSignatureCards.join(", ")}
        </p>
      ) : null}
      {previousRuns.length > 0 ? (
        <details>
          <summary>
            Earlier classifier provenance ({previousRuns.length})
          </summary>
          <ul>
            {previousRuns.map((run) => (
              <li key={run.id}>
                {run.classifierVersion}: {run.resultName} ({run.method})
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </section>
  );
}
