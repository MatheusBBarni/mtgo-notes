import { Tabs } from "@heroui/react";
import { useRef, useState, type FormEvent } from "react";

import {
  addAlias,
  applyMerge,
  applyUnmerge,
  createProfile,
  getEncounter,
  getProfile,
  previewDeletion,
  previewMerge,
  previewUnmerge,
  requestDeletion,
  saveObservation,
  searchHistory,
  setCardObservations,
  setTendencyTags,
  undoDeletion,
  updateObservation,
  updateProfile,
  type CardObservationInput,
  type DeletionPreview,
  type DeletionResult,
  type DeletionEntityType,
  type EncounterDetail,
  type HistoryHit,
  type MergePreview,
  type ProfileAggregate,
  type ProfileDetail,
  type UnmergePreview,
} from "../../lib/ipc/notebook";
import {
  Button,
  Panel,
  SelectField,
  StatusLabel,
  TextAreaField,
  TextField,
} from "../../ui/primitives";

type View = "notebook" | "history" | "identity" | "privacy";
type HistoryEntityType =
  "all" | "profile" | "alias" | "observation" | "deck" | "card" | "tag";
type HistoryCertainty = "any" | "observed" | "suspected";

const NOTEBOOK_VIEWS: readonly { label: string; value: View }[] = [
  { label: "Notebook", value: "notebook" },
  { label: "History", value: "history" },
  { label: "Identity", value: "identity" },
  { label: "Privacy", value: "privacy" },
];
const CARD_CERTAINTY_OPTIONS = [
  { label: "Observed", value: "observed" },
  { label: "Suspected", value: "suspected" },
] as const;
const HISTORY_ENTITY_OPTIONS = [
  { label: "All types", value: "all" },
  { label: "Profiles", value: "profile" },
  { label: "Aliases", value: "alias" },
  { label: "Observations", value: "observation" },
  { label: "Decks", value: "deck" },
  { label: "Cards", value: "card" },
  { label: "Tags", value: "tag" },
] as const;
const HISTORY_CERTAINTY_OPTIONS = [
  { label: "Any certainty", value: "any" },
  ...CARD_CERTAINTY_OPTIONS,
] as const;
const DELETION_SCOPE_OPTIONS = [
  { label: "One observation", value: "observation" },
  { label: "One encounter", value: "encounter" },
  { label: "One profile and its history", value: "profile" },
  { label: "Entire local notebook", value: "notebook" },
] as const;

export function NotebookWorkspace() {
  const [view, setView] = useState<View>("notebook");
  const [status, setStatus] = useState("Ready");
  const [error, setError] = useState<string>();
  const [profile, setProfile] = useState<ProfileAggregate>();

  return (
    <Tabs
      className="notebook-layout"
      onSelectionChange={(key) => {
        setView(key as View);
        setError(undefined);
      }}
      orientation="vertical"
      selectedKey={view}
      variant="secondary"
    >
      <Tabs.ListContainer className="min-w-0">
        <Tabs.List aria-label="Notebook sections" className="notebook-nav">
          {NOTEBOOK_VIEWS.map((item) => (
            <Tabs.Tab
              className="w-full justify-start"
              id={item.value}
              key={item.value}
            >
              {item.label}
              <Tabs.Indicator />
            </Tabs.Tab>
          ))}
        </Tabs.List>
      </Tabs.ListContainer>
      <Tabs.Panel className="notebook-content" id="notebook">
        <WorkspaceStatus error={error} status={status} />
        <NotebookPanel
          onError={setError}
          onProfile={setProfile}
          onStatus={setStatus}
          profile={profile}
        />
      </Tabs.Panel>
      <Tabs.Panel className="notebook-content" id="history">
        <WorkspaceStatus error={error} status={status} />
        <HistoryPanel onError={setError} onStatus={setStatus} />
      </Tabs.Panel>
      <Tabs.Panel className="notebook-content" id="identity">
        <WorkspaceStatus error={error} status={status} />
        <IdentityPanel onError={setError} onStatus={setStatus} />
      </Tabs.Panel>
      <Tabs.Panel className="notebook-content" id="privacy">
        <WorkspaceStatus error={error} status={status} />
        <PrivacyPanel onError={setError} onStatus={setStatus} />
      </Tabs.Panel>
    </Tabs>
  );
}

function WorkspaceStatus({
  error,
  status,
}: {
  error?: string;
  status: string;
}) {
  return (
    <div aria-atomic="true" aria-live="polite" className="notebook-status">
      <StatusLabel kind={error ? "error" : "source"} label={error ?? status} />
    </div>
  );
}

function NotebookPanel({
  onError,
  onProfile,
  onStatus,
  profile,
}: {
  onError: (message?: string) => void;
  onProfile: (profile: ProfileAggregate) => void;
  onStatus: (message: string) => void;
  profile?: ProfileAggregate;
}) {
  const [handle, setHandle] = useState("");
  const [alias, setAlias] = useState("");
  const [primaryHandleDraft, setPrimaryHandleDraft] = useState<{
    profileId: string;
    value: string;
  }>();
  const [encounterId, setEncounterId] = useState("");
  const [text, setText] = useState("");
  const [deckLabel, setDeckLabel] = useState("");
  const [cardName, setCardName] = useState("");
  const [certainty, setCertainty] = useState<"observed" | "suspected">(
    "observed",
  );
  const [context, setContext] = useState("");
  const [tags, setTags] = useState("");
  const [busy, setBusy] = useState(false);
  const observationRef = useRef<HTMLTextAreaElement>(null);
  const primaryHandle =
    profile && primaryHandleDraft?.profileId === profile.profile.id
      ? primaryHandleDraft.value
      : (profile?.profile.primaryHandle ?? "");

  async function create(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    onError(undefined);
    const result = await createProfile(handle);
    setBusy(false);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    onProfile(result.data);
    onStatus(`Profile ready: ${result.data.profile.primaryHandle}`);
    setHandle("");
  }

  async function aliasProfile(event: FormEvent) {
    event.preventDefault();
    if (!profile) return;
    setBusy(true);
    const result = await addAlias(profile.profile.id, alias);
    setBusy(false);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    onProfile(result.data);
    setAlias("");
    onStatus("Alias saved with exact matching.");
  }

  async function editProfile(event: FormEvent) {
    event.preventDefault();
    if (!profile) return;
    setBusy(true);
    onError(undefined);
    const result = await updateProfile(
      profile.profile.id,
      primaryHandle,
      profile.profile.revision,
    );
    setBusy(false);
    if (!result.ok) {
      onError(`${result.error.message} Refresh the profile before retrying.`);
      return;
    }
    onProfile(result.data);
    setPrimaryHandleDraft(undefined);
    onStatus("Primary handle updated; the previous handle remains an alias.");
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    const cards: CardObservationInput[] = cardName.trim()
      ? [
          {
            displayName: cardName,
            quantity: 1,
            certainty,
            context: context || undefined,
          },
        ]
      : [];
    setBusy(true);
    onError(undefined);
    const result = await saveObservation({
      encounterId,
      text,
      cards,
      tags: tags
        .split(",")
        .map((tag) => tag.trim())
        .filter(Boolean),
      userDeckLabel: deckLabel || undefined,
    });
    setBusy(false);
    if (!result.ok) {
      onError(`${result.error.message} Your draft is still here.`);
      observationRef.current?.focus();
      return;
    }
    setText("");
    setCardName("");
    setContext("");
    setTags("");
    onStatus("Observation saved with encounter provenance.");
    observationRef.current?.focus();
  }

  return (
    <div className="ui-stack">
      <Panel label="Opponent profiles">
        <form className="ui-stack" onSubmit={create}>
          <h2>Personal notebook</h2>
          <p className="notebook-hint">
            Handles keep their entered display form. Matching uses exact Unicode
            normalization only; similar names are never merged automatically.
          </p>
          <TextField
            label="Opponent handle"
            onChange={(event) => setHandle(event.currentTarget.value)}
            required
            value={handle}
          />
          <Button busy={busy} type="submit">
            Create or open profile
          </Button>
        </form>
        {profile ? (
          <div className="profile-summary" data-testid="profile-summary">
            <h3>{profile.profile.primaryHandle}</h3>
            <p>
              Revision {profile.profile.revision} · {profile.aliases.length}{" "}
              aliases
            </p>
            {profile.aliases.length > 0 ? (
              <ul aria-label="Profile aliases" className="notebook-chip-list">
                {profile.aliases.map((item) => (
                  <li key={item.id}>
                    {item.displayHandle} · {item.provenance}
                  </li>
                ))}
              </ul>
            ) : null}
            <form className="ui-actions" onSubmit={editProfile}>
              <TextField
                label="Primary handle"
                onChange={(event) => {
                  setPrimaryHandleDraft({
                    profileId: profile.profile.id,
                    value: event.currentTarget.value,
                  });
                }}
                required
                value={primaryHandle}
              />
              <Button busy={busy} type="submit" variant="secondary">
                Update primary handle
              </Button>
            </form>
            <form className="ui-actions" onSubmit={aliasProfile}>
              <TextField
                label="New exact alias"
                onChange={(event) => setAlias(event.currentTarget.value)}
                required
                value={alias}
              />
              <Button busy={busy} type="submit" variant="secondary">
                Add alias
              </Button>
            </form>
          </div>
        ) : (
          <p className="notebook-empty">No profile selected yet.</p>
        )}
      </Panel>

      <Panel label="Observation editor">
        <form className="ui-stack" onSubmit={save}>
          <h2>Add an observation</h2>
          <TextField
            label="Encounter ID"
            onChange={(event) => setEncounterId(event.currentTarget.value)}
            required
            value={encounterId}
          />
          <TextAreaField
            className="notebook-textarea"
            inputId="notebook-observation"
            label="Observation"
            onChange={(event) => setText(event.currentTarget.value)}
            ref={observationRef}
            required
            value={text}
          />
          <details>
            <summary>Optional structure</summary>
            <div className="notebook-form-grid">
              <TextField
                label="User-entered deck label"
                onChange={(event) => setDeckLabel(event.currentTarget.value)}
                value={deckLabel}
              />
              <TextField
                label="Card name"
                onChange={(event) => setCardName(event.currentTarget.value)}
                value={cardName}
              />
              <SelectField
                label="Card certainty"
                name="card-certainty"
                onChange={setCertainty}
                options={CARD_CERTAINTY_OPTIONS}
                value={certainty}
              />
              <TextField
                label="Card context"
                onChange={(event) => setContext(event.currentTarget.value)}
                value={context}
              />
              <TextField
                label="Custom tendency tags (comma-separated)"
                onChange={(event) => setTags(event.currentTarget.value)}
                value={tags}
              />
            </div>
          </details>
          <Button busy={busy} type="submit">
            Save observation
          </Button>
        </form>
      </Panel>
    </div>
  );
}

function HistoryPanel({
  onError,
  onStatus,
}: {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [entityType, setEntityType] = useState<HistoryEntityType>("all");
  const [certainty, setCertainty] = useState<HistoryCertainty>("any");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [profileId, setProfileId] = useState("");
  const [encounterId, setEncounterId] = useState("");
  const [items, setItems] = useState<HistoryHit[]>([]);
  const [cursor, setCursor] = useState<string>();
  const [profileDetail, setProfileDetail] = useState<ProfileDetail>();
  const [encounterDetail, setEncounterDetail] = useState<EncounterDetail>();
  const [busy, setBusy] = useState(false);

  async function runSearch(nextCursor?: string, append = false) {
    setBusy(true);
    onError(undefined);
    const result = await searchHistory({
      text: query,
      cursor: nextCursor,
      filters: {
        entityTypes: entityType === "all" ? [] : [entityType],
        dateFrom: dateFrom
          ? new Date(`${dateFrom}T00:00:00`).getTime()
          : undefined,
        dateTo: dateTo
          ? new Date(`${dateTo}T23:59:59.999`).getTime()
          : undefined,
        certainty:
          certainty === "observed" || certainty === "suspected"
            ? certainty
            : undefined,
      },
    });
    setBusy(false);
    if (!result.ok) {
      setItems([]);
      setCursor(undefined);
      onError(result.error.message);
      return;
    }
    setItems((current) =>
      append ? [...current, ...result.data.items] : result.data.items,
    );
    setCursor(result.data.nextCursor);
    onStatus(
      result.data.items.length === 0
        ? "No notebook history matched."
        : `${result.data.items.length} history results loaded.`,
    );
  }

  async function search(event: FormEvent) {
    event.preventDefault();
    setProfileDetail(undefined);
    setEncounterDetail(undefined);
    await runSearch();
  }

  async function openProfile() {
    setBusy(true);
    onError(undefined);
    const result = await getProfile(profileId);
    setBusy(false);
    if (!result.ok) {
      setProfileDetail(undefined);
      onError(result.error.message);
      return;
    }
    setProfileDetail(result.data);
    setEncounterDetail(undefined);
    if (result.data.canonicalProfileId) {
      onStatus("This profile was merged; showing its canonical profile.");
    } else {
      onStatus("Profile timeline loaded from the local notebook.");
    }
  }

  async function openEncounter(id = encounterId) {
    setBusy(true);
    onError(undefined);
    const result = await getEncounter(id);
    setBusy(false);
    if (!result.ok) {
      setEncounterDetail(undefined);
      onError(result.error.message);
      return;
    }
    setEncounterDetail(result.data);
    setProfileDetail(undefined);
    setEncounterId(id);
    onStatus("Encounter detail loaded with source and edit provenance.");
  }

  return (
    <div className="ui-stack">
      <Panel label="Offline notebook history">
        <form className="ui-stack" onSubmit={search}>
          <h2>History and search</h2>
          <p className="notebook-hint">
            History is read from the encrypted local notebook. The host denies
            every history command whenever gameplay disclosure is restricted.
          </p>
          <TextField
            label="Search handles, aliases, notes, decks, cards, or tags"
            onChange={(event) => setQuery(event.currentTarget.value)}
            required
            value={query}
          />
          <div className="notebook-form-grid">
            <SelectField
              label="Result type"
              name="history-entity-type"
              onChange={setEntityType}
              options={HISTORY_ENTITY_OPTIONS}
              value={entityType}
            />
            <SelectField
              label="Card certainty"
              name="history-certainty"
              onChange={setCertainty}
              options={HISTORY_CERTAINTY_OPTIONS}
              value={certainty}
            />
            <TextField
              label="From date"
              onChange={(event) => setDateFrom(event.currentTarget.value)}
              type="date"
              value={dateFrom}
            />
            <TextField
              label="Through date"
              onChange={(event) => setDateTo(event.currentTarget.value)}
              type="date"
              value={dateTo}
            />
          </div>
          <Button busy={busy} type="submit">
            Search local history
          </Button>
        </form>
        {items.length === 0 ? (
          <p className="notebook-empty">No results to show.</p>
        ) : (
          <ol className="history-list">
            {items.map((item) => (
              <li key={`${item.entityType}:${item.entityId}`}>
                <strong>{item.content}</strong>
                <span>
                  {item.entityType} · {new Date(item.sortMs).toLocaleString()}
                </span>
              </li>
            ))}
          </ol>
        )}
        {cursor ? (
          <Button
            busy={busy}
            onClick={() => runSearch(cursor, true)}
            variant="secondary"
          >
            Load next stable page
          </Button>
        ) : null}
      </Panel>

      <Panel label="Notebook detail">
        <div className="notebook-form-grid">
          <div className="ui-stack">
            <TextField
              label="Profile ID"
              onChange={(event) => setProfileId(event.currentTarget.value)}
              value={profileId}
            />
            <Button
              busy={busy}
              disabled={!profileId}
              onClick={openProfile}
              variant="secondary"
            >
              Open profile timeline
            </Button>
          </div>
          <div className="ui-stack">
            <TextField
              label="Encounter ID"
              onChange={(event) => setEncounterId(event.currentTarget.value)}
              value={encounterId}
            />
            <Button
              busy={busy}
              disabled={!encounterId}
              onClick={() => openEncounter()}
              variant="secondary"
            >
              Open encounter
            </Button>
          </div>
        </div>
        {profileDetail ? (
          <ProfileTimeline
            detail={profileDetail}
            onOpenEncounter={openEncounter}
          />
        ) : null}
        {encounterDetail ? (
          <EncounterEditor
            detail={encounterDetail}
            onChange={setEncounterDetail}
            onError={onError}
            onStatus={onStatus}
          />
        ) : null}
      </Panel>
    </div>
  );
}

function ProfileTimeline({
  detail,
  onOpenEncounter,
}: {
  detail: ProfileDetail;
  onOpenEncounter: (id: string) => void;
}) {
  return (
    <section aria-label="Profile timeline" className="profile-summary">
      <h3>{detail.profile.profile.primaryHandle}</h3>
      {detail.canonicalProfileId ? (
        <p>Canonical profile: {detail.canonicalProfileId}</p>
      ) : null}
      {detail.lastDeckSeen ? (
        <div className="deck-provenance">
          <strong>Last deck seen: {detail.lastDeckSeen.label}</strong>
          <p>
            {detail.lastDeckSeen.sourceClass} ·{" "}
            {detail.lastDeckSeen.sourceLabel} · {detail.lastDeckSeen.format} ·{" "}
            {new Date(detail.lastDeckSeen.seenAt).toLocaleString()}
          </p>
        </div>
      ) : (
        <p className="notebook-empty">No confirmed deck history.</p>
      )}
      <ol className="history-list">
        {detail.encounters.map((encounter) => (
          <li key={encounter.id}>
            <strong>
              {encounter.format} · {encounter.status}
            </strong>
            <span>
              {new Date(encounter.startedAt).toLocaleString()} ·{" "}
              {encounter.source} · {encounter.observationCount} observations
            </span>
            <Button
              onClick={() => onOpenEncounter(encounter.id)}
              variant="secondary"
            >
              Review encounter
            </Button>
          </li>
        ))}
      </ol>
    </section>
  );
}

function EncounterEditor({
  detail,
  onChange,
  onError,
  onStatus,
}: {
  detail: EncounterDetail;
  onChange: (detail: EncounterDetail) => void;
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
}) {
  return (
    <section aria-label="Encounter observations" className="profile-summary">
      <h3>
        {detail.summary.format} encounter · {detail.summary.status}
      </h3>
      <p>
        {new Date(detail.summary.startedAt).toLocaleString()} ·{" "}
        {detail.summary.source}
        {detail.summary.incompleteReason
          ? ` · incomplete: ${detail.summary.incompleteReason}`
          : ""}
      </p>
      {detail.observations.length === 0 ? (
        <p className="notebook-empty">This encounter has no observations.</p>
      ) : (
        detail.observations.map((observation) => (
          <ObservationEditor
            key={observation.id}
            observation={observation}
            onChange={(next) =>
              onChange({
                ...detail,
                observations: detail.observations.map((current) =>
                  current.id === next.id ? next : current,
                ),
              })
            }
            onError={onError}
            onStatus={onStatus}
          />
        ))
      )}
    </section>
  );
}

function ObservationEditor({
  observation,
  onChange,
  onError,
  onStatus,
}: {
  observation: EncounterDetail["observations"][number];
  onChange: (observation: EncounterDetail["observations"][number]) => void;
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
}) {
  const [text, setText] = useState(observation.text);
  const [cardName, setCardName] = useState(
    observation.cards[0]?.displayName ?? "",
  );
  const [certainty, setCertainty] = useState<"observed" | "suspected">(
    observation.cards[0]?.certainty ?? "observed",
  );
  const [context, setContext] = useState(observation.cards[0]?.context ?? "");
  const [tags, setTags] = useState(
    observation.tags.map((tag) => tag.displayLabel).join(", "),
  );
  const [busy, setBusy] = useState(false);

  async function run(
    operation: Promise<Awaited<ReturnType<typeof updateObservation>>>,
    message: string,
  ) {
    setBusy(true);
    onError(undefined);
    const result = await operation;
    setBusy(false);
    if (!result.ok) {
      onError(
        `${result.error.message} Refresh this encounter before retrying.`,
      );
      return;
    }
    onChange(result.data);
    onStatus(message);
  }

  return (
    <article className="observation-editor">
      <p>
        Encounter provenance:{" "}
        {new Date(observation.encounterStartedAt).toLocaleString()} · revision{" "}
        {observation.revision}
        {observation.editedAt ? " · edited" : ""}
      </p>
      <TextAreaField
        className="notebook-textarea"
        label="Observation text"
        onChange={(event) => setText(event.currentTarget.value)}
        value={text}
      />
      <Button
        busy={busy}
        onClick={() =>
          run(
            updateObservation(observation.id, text, observation.revision),
            "Observation text updated with provenance intact.",
          )
        }
      >
        Save text edit
      </Button>
      <details>
        <summary>Edit optional structure</summary>
        <div className="notebook-form-grid">
          <TextField
            label="Card name"
            onChange={(event) => setCardName(event.currentTarget.value)}
            value={cardName}
          />
          <SelectField
            label="Card certainty"
            onChange={setCertainty}
            options={CARD_CERTAINTY_OPTIONS}
            value={certainty}
          />
          <TextField
            label="Card context"
            onChange={(event) => setContext(event.currentTarget.value)}
            value={context}
          />
          <TextField
            label="Tendency tags"
            onChange={(event) => setTags(event.currentTarget.value)}
            value={tags}
          />
        </div>
        <div className="ui-actions">
          <Button
            busy={busy}
            onClick={() =>
              run(
                setCardObservations(
                  observation.id,
                  cardName
                    ? [
                        {
                          displayName: cardName,
                          quantity: 1,
                          certainty,
                          context: context || undefined,
                        },
                      ]
                    : [],
                  observation.revision,
                ),
                "Card observations replaced atomically.",
              )
            }
            variant="secondary"
          >
            Save cards
          </Button>
          <Button
            busy={busy}
            onClick={() =>
              run(
                setTendencyTags(
                  observation.id,
                  tags
                    .split(",")
                    .map((tag) => tag.trim())
                    .filter(Boolean),
                  observation.revision,
                ),
                "Tendency tags replaced atomically.",
              )
            }
            variant="secondary"
          >
            Save tags
          </Button>
        </div>
      </details>
    </article>
  );
}

function IdentityPanel({
  onError,
  onStatus,
}: {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
}) {
  const [left, setLeft] = useState("");
  const [right, setRight] = useState("");
  const [primary, setPrimary] = useState("");
  const [preview, setPreview] = useState<MergePreview>();
  const [mergeId, setMergeId] = useState("");
  const [unmergePreview, setUnmergePreview] = useState<UnmergePreview>();

  async function build(event: FormEvent) {
    event.preventDefault();
    const result = await previewMerge(left, right, primary);
    if (!result.ok) {
      onError(result.error.message);
      setPreview(undefined);
      return;
    }
    setPreview(result.data);
    onStatus("Merge preview is ready; no data has changed.");
  }

  async function apply() {
    if (!preview) return;
    const result = await applyMerge(preview);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    setPreview(undefined);
    setMergeId(result.data.mergeId);
    onStatus(`Profiles merged. Undo record: ${result.data.mergeId}`);
  }

  async function buildUnmerge(event: FormEvent) {
    event.preventDefault();
    const result = await previewUnmerge(mergeId);
    if (!result.ok) {
      onError(result.error.message);
      setUnmergePreview(undefined);
      return;
    }
    setUnmergePreview(result.data);
    onStatus("Unmerge assignments previewed; no data has changed.");
  }

  async function unmerge() {
    if (!unmergePreview) return;
    const result = await applyUnmerge(unmergePreview);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    setUnmergePreview(undefined);
    onStatus("Profiles restored according to the confirmed assignment plan.");
  }

  return (
    <div className="ui-stack">
      <Panel label="Identity correction">
        <form className="ui-stack" onSubmit={build}>
          <h2>Preview reversible merge</h2>
          <div className="notebook-form-grid">
            <TextField
              label="First profile ID"
              onChange={(event) => setLeft(event.currentTarget.value)}
              required
              value={left}
            />
            <TextField
              label="Second profile ID"
              onChange={(event) => setRight(event.currentTarget.value)}
              required
              value={right}
            />
            <TextField
              label="Primary profile ID"
              onChange={(event) => setPrimary(event.currentTarget.value)}
              required
              value={primary}
            />
          </div>
          <Button type="submit">Preview merge</Button>
        </form>
        {preview ? (
          <div className="merge-preview">
            <h3>
              {preview.secondaryHandle} → {preview.primaryHandle}
            </h3>
            <dl className="count-grid">
              {Object.entries(preview.affected).map(([label, count]) => (
                <div key={label}>
                  <dt>{label}</dt>
                  <dd>{count}</dd>
                </div>
              ))}
            </dl>
            <p>
              Conflicts: {preview.conflictCount}. Encounters, observations, deck
              provenance, and timestamps remain intact.
            </p>
            {preview.conflicts.length > 0 ? (
              <ul aria-label="Merge conflicts">
                {preview.conflicts.map((conflict) => (
                  <li key={conflict}>{conflict}</li>
                ))}
              </ul>
            ) : null}
            <ul aria-label="Merge consequences">
              {preview.irreversibleConsequences.map((consequence) => (
                <li key={consequence}>{consequence}</li>
              ))}
            </ul>
            <Button onClick={apply}>Confirm merge</Button>
          </div>
        ) : null}
      </Panel>
      <Panel label="Reverse a merge">
        <form className="ui-stack" onSubmit={buildUnmerge}>
          <TextField
            label="Merge undo record ID"
            onChange={(event) => setMergeId(event.currentTarget.value)}
            required
            value={mergeId}
          />
          <Button type="submit" variant="secondary">
            Preview unmerge assignments
          </Button>
        </form>
        {unmergePreview ? (
          <div className="merge-preview">
            <h3>Unmerge assignment plan</h3>
            <p>
              Restore {unmergePreview.restoredEncounters} encounters and{" "}
              {unmergePreview.restoredDecks} decks.{" "}
              {unmergePreview.postMergeEncounters} post-merge encounters and{" "}
              {unmergePreview.postMergeDecks} post-merge decks will{" "}
              {unmergePreview.proposedPostMergeAssignment ===
              "retain_with_primary"
                ? " remain with the primary profile"
                : " follow the selected assignment"}
              .
            </p>
            <Button onClick={unmerge}>Apply confirmed unmerge</Button>
          </div>
        ) : null}
      </Panel>
    </div>
  );
}

function PrivacyPanel({
  onError,
  onStatus,
}: {
  onError: (message?: string) => void;
  onStatus: (message: string) => void;
}) {
  const [entityType, setEntityType] = useState<DeletionEntityType>("profile");
  const [entityId, setEntityId] = useState("");
  const [preview, setPreview] = useState<DeletionPreview>();
  const [confirmation, setConfirmation] = useState("");
  const [pending, setPending] = useState<DeletionResult>();

  async function build(event: FormEvent) {
    event.preventDefault();
    const targetId = entityType === "notebook" ? "notebook" : entityId;
    const result = await previewDeletion(entityType, targetId);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    setPreview(result.data);
    setConfirmation("");
    onStatus("Deletion scope previewed; nothing has been removed.");
  }

  async function remove() {
    if (!preview) return;
    const result = await requestDeletion(preview, confirmation);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    setPending(result.data);
    setPreview(undefined);
    onStatus("Selected data is hidden now. Undo remains available briefly.");
  }

  async function undo() {
    if (!pending) return;
    const result = await undoDeletion(pending);
    if (!result.ok) {
      onError(result.error.message);
      return;
    }
    setPending(undefined);
    onStatus("Deletion undone; local search index restored.");
  }

  return (
    <Panel label="Privacy and deletion">
      <form className="ui-stack" onSubmit={build}>
        <h2>Delete private notebook data</h2>
        <SelectField
          label="Deletion scope"
          name="deletion-entity-type"
          onChange={(value) => {
            setEntityType(value);
            setPreview(undefined);
          }}
          options={DELETION_SCOPE_OPTIONS}
          value={entityType}
        />
        <TextField
          disabled={entityType === "notebook"}
          label={entityType === "notebook" ? "Notebook" : "Entity ID"}
          onChange={(event) => setEntityId(event.currentTarget.value)}
          required={entityType !== "notebook"}
          value={entityType === "notebook" ? "notebook" : entityId}
        />
        <Button type="submit" variant="destructive">
          Preview deletion
        </Button>
      </form>
      {preview ? (
        <div className="deletion-preview">
          <h3>Affected scope: {preview.displayName}</h3>
          <p>
            {preview.counts.profiles} profiles, {preview.counts.aliases}{" "}
            aliases, {preview.counts.encounters} encounters,{" "}
            {preview.counts.observations} observations, {preview.counts.decks}{" "}
            decks, and {preview.counts.publicSnapshots} public snapshots.
          </p>
          {preview.dependencies.length > 0 ? (
            <div className="portability-warning" role="alert">
              Resolve before deletion: {preview.dependencies.join(", ")}
            </div>
          ) : null}
          <TextField
            label={`Type exactly: ${preview.confirmation}`}
            onChange={(event) => setConfirmation(event.currentTarget.value)}
            value={confirmation}
          />
          <Button
            disabled={confirmation !== preview.confirmation}
            onClick={remove}
            variant="destructive"
          >
            Confirm scoped deletion
          </Button>
        </div>
      ) : null}
      {pending ? (
        <div className="undo-banner" role="status">
          <p>
            Deleted until purge. Undo deadline:{" "}
            {new Date(pending.undoDeadline).toLocaleTimeString()}
          </p>
          <Button onClick={undo} variant="secondary">
            Undo deletion
          </Button>
        </div>
      ) : null}
    </Panel>
  );
}
