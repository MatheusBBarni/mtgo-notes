import { useMemo, useState, type FormEvent } from "react";

import {
  confirmPlayerDeletion,
  cancelPlayerLookup,
  grantPlayerConsent,
  importPlayerEvidence,
  operationKey,
  openPlayerSource,
  previewManualPlayerEvidence,
  previewPlayerDeletion,
  refreshPlayerResults,
  revokePlayerConsent,
  savePlayerIdentity,
  startPlayerLookup,
  type ManualEvidenceInput,
  type PlayerCandidateView,
  type PlayerEvidenceView,
  type PlayerManualPreviewView,
  type PlayerRoute,
} from "../../lib/ipc/player";
import {
  Button,
  Checkbox,
  Panel,
  StatusLabel,
  TextField,
} from "../../ui/primitives";
import {
  FIRST_USE_PLAYER_VIEW,
  usePlayerWorkspace,
} from "./usePlayerWorkspace";

const DISCLOSURES: Record<PlayerRoute, { title: string; version: string }> = {
  census_mocs: {
    title: "Census MOCS leaderboard",
    version: "player-census-v1",
  },
  official_mtgo_browser: {
    title: "Official MTGO decklists",
    version: "player-official-v1",
  },
  mtg_top8_browser: {
    title: "MTGTop8 reference",
    version: "player-mtgtop8-v1",
  },
};

export function PlayerWorkspace() {
  const { view, error, refresh } = usePlayerWorkspace();
  const [nickname, setNickname] = useState("");
  const [busy, setBusy] = useState(false);
  const [localError, setLocalError] = useState<string>();
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [manual, setManual] = useState<ManualEvidenceInput>({
    eventTitle: "",
    eventDate: "",
    format: "",
    sourceNickname: "",
    attributionUrl: "",
    contents: "reference_only",
    cards: [],
  });
  const [manualPreview, setManualPreview] = useState<PlayerManualPreviewView>();
  const [deletion, setDeletion] = useState(view.deletion);
  const identity = view.identity;
  const displayError = localError ?? error;
  const hasEvidence = view.evidence.items.length > 0;

  async function saveIdentity(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setLocalError(undefined);
    const result = await savePlayerIdentity({
      displayNickname: nickname,
      expectedRevision: identity?.revision,
      idempotencyKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) {
      setLocalError(result.error.message);
      return;
    }
    setNickname("");
    await refresh();
  }

  async function startLookup() {
    if (!identity) return;
    const census = view.sources.find(
      (source) => source.route === "census_mocs",
    );
    if (!census?.consentGranted) {
      setLocalError("Review and grant Census disclosure before lookup.");
      return;
    }
    setBusy(true);
    const result = await startPlayerLookup({
      identityRevision: identity.revision,
      consentVersion: census.disclosureVersion ?? "",
      fieldsDigest: "0".repeat(64),
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else await refresh();
  }

  async function cancelLookup() {
    const activeOperationKey = view.lookup.operationKey;
    if (!activeOperationKey) return;
    setBusy(true);
    const result = await cancelPlayerLookup(activeOperationKey);
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else await refresh();
  }

  async function openSource(route: Exclude<PlayerRoute, "census_mocs">) {
    setBusy(true);
    const result = await openPlayerSource({
      route,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else setLocalError(undefined);
  }

  async function importCandidate(candidate: PlayerCandidateView) {
    const fields = selected[candidate.sourceKey]
      ? candidate.approvedFields.reduce<Record<string, boolean>>(
          (all, field) => {
            all[field] = true;
            return all;
          },
          {},
        )
      : {};
    if (!fields.source_nickname || !fields.attribution_url) {
      setLocalError("Source identity and attribution are required.");
      return;
    }
    setBusy(true);
    const result = await importPlayerEvidence({
      token: candidate.token ?? candidate.previewDigest,
      previewDigest: candidate.previewDigest,
      selectedFields: fields,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else {
      setSelected({});
      await refresh();
    }
  }

  async function importManualPreview() {
    if (!manualPreview) return;
    setBusy(true);
    const result = await importPlayerEvidence({
      token: manualPreview.token,
      previewDigest: manualPreview.evidence.previewDigest,
      selectedFields: manualPreview.evidence.selectedFields,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else {
      setManualPreview(undefined);
      await refresh();
    }
  }

  async function createManualPreview(event: FormEvent) {
    event.preventDefault();
    if (!identity) return;
    setBusy(true);
    const result = await previewManualPlayerEvidence({
      input: manual,
      identityRevision: identity.revision,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else setManualPreview(result.data);
  }

  async function createDeletionPreview() {
    if (!identity) return;
    setBusy(true);
    const result = await previewPlayerDeletion({
      target: "identity",
      targetId: identity.id,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else setDeletion(result.data);
  }

  async function confirmDeletion() {
    if (!deletion) return;
    setBusy(true);
    const result = await confirmPlayerDeletion({
      token: deletion.token,
      digest: deletion.digest,
      operationKey: operationKey(),
    });
    setBusy(false);
    if (!result.ok) setLocalError(result.error.message);
    else {
      setDeletion(null);
      await refresh();
    }
  }

  return (
    <section aria-label="Player workspace" className="player-workspace">
      <div aria-atomic="true" aria-live="polite" className="player-status">
        <StatusLabel
          kind={displayError ? "error" : "source"}
          label={displayError ?? view.lookup.message}
        />
      </div>
      <div className="player-layout">
        <div className="player-main ui-stack">
          <Panel label="Player identity">
            {!identity ? (
              <form className="ui-stack" onSubmit={saveIdentity}>
                <p className="muted-copy">
                  Optional local identity for reviewing public results. Opening
                  this tab never starts a lookup or grants consent.
                </p>
                <TextField
                  label="MTGO nickname"
                  maxLength={128}
                  onChange={(event) => setNickname(event.target.value)}
                  required
                  value={nickname}
                />
                <Button busy={busy} type="submit">
                  Save identity
                </Button>
              </form>
            ) : (
              <div className="ui-stack">
                <p>
                  <strong>{identity.displayNickname}</strong>{" "}
                  <span className="muted-copy">(local only)</span>
                </p>
                <form className="ui-actions" onSubmit={saveIdentity}>
                  <TextField
                    aria-label="Edit nickname"
                    label="Edit nickname"
                    maxLength={128}
                    onChange={(event) => setNickname(event.target.value)}
                    value={nickname}
                  />
                  <Button busy={busy} type="submit" variant="secondary">
                    Save edit
                  </Button>
                </form>
                <p className="player-warning">
                  Changing the display spelling does not rewrite historical
                  source nicknames.
                </p>
              </div>
            )}
          </Panel>
          {!identity ? (
            <Panel label="Public sources and consent">
              <p className="muted-copy">
                Public sources are disabled until you save a local identity and
                review each disclosure. Opening this tab performs no lookup,
                browser handoff, or consent action.
              </p>
            </Panel>
          ) : null}
          {identity ? (
            <>
              <PlayerSourceControls
                busy={busy}
                onConsent={async (route) => {
                  const disclosure = DISCLOSURES[route];
                  const result = await grantPlayerConsent({
                    route,
                    disclosureVersion: disclosure.version,
                    fieldsDigest: "0".repeat(64),
                    idempotencyKey: operationKey(),
                  });
                  if (!result.ok) setLocalError(result.error.message);
                  else await refresh();
                }}
                onRevoke={async (route) => {
                  const result = await revokePlayerConsent({
                    route,
                    idempotencyKey: operationKey(),
                  });
                  if (!result.ok) setLocalError(result.error.message);
                  else await refresh();
                }}
                onOpen={openSource}
                sources={view.sources}
              />
              <Panel label="Public results">
                <div className="ui-stack">
                  <div className="ui-actions">
                    <Button busy={busy} onClick={startLookup}>
                      Look up exact nickname
                    </Button>
                    <Button
                      busy={busy}
                      onClick={() =>
                        void refreshPlayerResults({
                          identityRevision: identity.revision,
                          operationKey: operationKey(),
                        }).then(() => refresh())
                      }
                      variant="secondary"
                    >
                      Refresh results
                    </Button>
                    {view.lookup.state === "loading" ? (
                      <Button
                        busy={busy}
                        disabled={!view.lookup.operationKey}
                        onClick={() => void cancelLookup()}
                        variant="secondary"
                      >
                        Cancel lookup
                      </Button>
                    ) : null}
                  </div>
                  <p className="muted-copy">
                    Provider status, routes, and provenance come from the host.
                    Saved evidence stays visible while a lookup is loading or
                    unavailable.
                  </p>
                  {view.lookup.candidates.map((candidate) => (
                    <PlayerCandidateCard
                      candidate={candidate}
                      key={`${candidate.sourceKey}:${candidate.sourceDigest}`}
                      onImport={() => void importCandidate(candidate)}
                      selected={Boolean(selected[candidate.sourceKey])}
                      onSelected={(checked) =>
                        setSelected((current) => ({
                          ...current,
                          [candidate.sourceKey]: checked,
                        }))
                      }
                    />
                  ))}
                </div>
              </Panel>
              <Panel label="Manual official result">
                <form className="ui-stack" onSubmit={createManualPreview}>
                  <TextField
                    label="Event title"
                    maxLength={200}
                    onChange={(event) =>
                      setManual((current) => ({
                        ...current,
                        eventTitle: event.target.value,
                      }))
                    }
                    value={manual.eventTitle}
                  />
                  <div className="player-form-grid">
                    <TextField
                      label="Event date"
                      maxLength={10}
                      onChange={(event) =>
                        setManual((current) => ({
                          ...current,
                          eventDate: event.target.value,
                        }))
                      }
                      value={manual.eventDate}
                    />
                    <TextField
                      label="Format"
                      maxLength={64}
                      onChange={(event) =>
                        setManual((current) => ({
                          ...current,
                          format: event.target.value,
                        }))
                      }
                      value={manual.format}
                    />
                  </div>
                  <TextField
                    label="Exact source nickname"
                    maxLength={128}
                    onChange={(event) =>
                      setManual((current) => ({
                        ...current,
                        sourceNickname: event.target.value,
                      }))
                    }
                    value={manual.sourceNickname}
                  />
                  <TextField
                    label="Official artifact URL"
                    onChange={(event) =>
                      setManual((current) => ({
                        ...current,
                        attributionUrl: event.target.value,
                      }))
                    }
                    value={manual.attributionUrl}
                  />
                  <Button busy={busy} type="submit" variant="secondary">
                    Preview without fetching
                  </Button>
                  {manualPreview ? (
                    <div role="status" className="ui-actions">
                      <span>
                        Manual preview ready:{" "}
                        {manualPreview.evidence.sourceNickname}. Select it
                        explicitly to import.
                      </span>
                      <Button
                        busy={busy}
                        onClick={() => void importManualPreview()}
                        type="button"
                        variant="secondary"
                      >
                        Import manual result
                      </Button>
                    </div>
                  ) : null}
                </form>
              </Panel>
            </>
          ) : null}
        </div>
        <aside
          className="player-evidence ui-stack"
          aria-label="Saved Player evidence"
        >
          <PlayerEvidenceList evidence={view.evidence.items} />
          {hasEvidence ? (
            <p className="muted-copy">
              Evidence remains local and readable after consent revocation.
            </p>
          ) : (
            <p className="muted-copy">No saved Player evidence yet.</p>
          )}
          {identity ? (
            <Button
              busy={busy}
              onClick={createDeletionPreview}
              variant="destructive"
            >
              Delete Player data
            </Button>
          ) : null}
          {deletion ? (
            <PlayerDeletionDialog
              busy={busy}
              deletion={deletion}
              onCancel={() => setDeletion(null)}
              onConfirm={confirmDeletion}
            />
          ) : null}
        </aside>
      </div>
    </section>
  );
}

export function PlayerSourceControls({
  sources,
  busy,
  onConsent,
  onRevoke,
  onOpen,
}: {
  sources: ReturnType<typeof usePlayerWorkspace>["view"]["sources"];
  busy: boolean;
  onConsent: (route: PlayerRoute) => Promise<void>;
  onRevoke: (route: PlayerRoute) => Promise<void>;
  onOpen: (route: Exclude<PlayerRoute, "census_mocs">) => Promise<void>;
}) {
  return (
    <Panel label="Public sources and consent">
      <div className="ui-stack">
        {(
          [
            "census_mocs",
            "official_mtgo_browser",
            "mtg_top8_browser",
          ] as PlayerRoute[]
        ).map((route) => {
          const source = sources.find((item) => item.route === route);
          const disclosure = DISCLOSURES[route];
          return (
            <div className="player-source-row" key={route}>
              <div>
                <strong>{disclosure.title}</strong>
                <span className="muted-copy">
                  {" "}
                  {source?.availability ?? "disabled"}; consent{" "}
                  {source?.consentGranted ? "granted" : "not granted"}
                </span>
              </div>
              <div className="ui-actions">
                {!source?.consentGranted ? (
                  <Button
                    busy={busy}
                    onClick={() => onConsent(route)}
                    variant="secondary"
                  >
                    Review and grant
                  </Button>
                ) : (
                  <Button
                    busy={busy}
                    onClick={() => onRevoke(route)}
                    variant="secondary"
                  >
                    Revoke
                  </Button>
                )}
                {route !== "census_mocs" ? (
                  <Button
                    busy={busy}
                    disabled={!source?.consentGranted}
                    onClick={() => void onOpen(route)}
                    variant="secondary"
                  >
                    Open source
                  </Button>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
    </Panel>
  );
}

export function PlayerCandidateCard({
  candidate,
  selected,
  onSelected,
  onImport,
}: {
  candidate: PlayerCandidateView;
  selected: boolean;
  onSelected: (value: boolean) => void;
  onImport: () => void;
}) {
  const fields = useMemo(
    () => new Set(candidate.approvedFields),
    [candidate.approvedFields],
  );
  return (
    <article className="player-candidate">
      <Checkbox checked={selected} onChange={onSelected}>
        <strong>{candidate.sourceNickname}</strong>
      </Checkbox>
      <p className="muted-copy">
        Source identity and attribution are always retained. Approved fields:{" "}
        {[...fields].join(", ")}.
      </p>
      <Button disabled={!selected} onClick={onImport} variant="secondary">
        Import selected result
      </Button>
    </article>
  );
}

export function PlayerEvidenceList({
  evidence,
}: {
  evidence: PlayerEvidenceView[];
}) {
  return (
    <Panel label="Saved evidence">
      {evidence.length === 0 ? (
        <p className="muted-copy">
          Saved public results will appear here with attribution and version
          history.
        </p>
      ) : (
        <ol className="player-evidence-list">
          {evidence.map((item) => (
            <li key={item.id}>
              <strong>{item.sourceNickname}</strong>
              <span>
                {item.kind}; {item.provenanceMode}
              </span>
              <a href={item.attributionUrl} rel="noreferrer" target="_blank">
                Open attribution
              </a>
              {item.classification ? (
                <span>Classification: {item.classification.resultName}</span>
              ) : (
                <span>Unclassified</span>
              )}
            </li>
          ))}
        </ol>
      )}
    </Panel>
  );
}

export function PlayerDeletionDialog({
  deletion,
  busy,
  onCancel,
  onConfirm,
}: {
  deletion: NonNullable<
    ReturnType<typeof usePlayerWorkspace>["view"]["deletion"]
  >;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => Promise<void>;
}) {
  return (
    <div
      aria-label="Confirm Player deletion"
      className="player-deletion-dialog"
      role="dialog"
    >
      <h3>Delete Player data?</h3>
      <p>
        This removes the bound Player identity and {deletion.counts.evidence}{" "}
        evidence record(s). A content-free tombstone prevents accidental merge
        resurrection.
      </p>
      <div className="ui-actions">
        <Button onClick={onCancel} variant="secondary">
          Cancel
        </Button>
        <Button
          busy={busy}
          onClick={() => void onConfirm()}
          variant="destructive"
        >
          Confirm deletion
        </Button>
      </div>
    </div>
  );
}

export { FIRST_USE_PLAYER_VIEW };
