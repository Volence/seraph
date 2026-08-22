import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import type { Track, SongMetadata, SelectedRegion, SelectedInstrument } from "../types/model";
import { DEFAULT_FM_MODULATOR, DEFAULT_FM_CARRIER, channelTypeOf } from "../types/model";
import { useArrangementZoom } from "../hooks/useArrangementZoom";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import { TrackHeader, channelLevelIndex } from "./TrackHeader";
import { TimelineRuler } from "./TimelineRuler";
import { TimelineCanvas } from "./TimelineCanvas";
import { AddTrackDialog } from "./AddTrackDialog";
import * as ipc from "../api/ipc";
import * as grid from "../utils/grid";
import { SONG_REVERTED_EVENT, isEditableTarget } from "../utils/keyboard";
import { pianoRollNoteSelectionActive } from "../utils/noteSelection";
import { copyRegions, getRegionClipboard, lastCopiedKind } from "../utils/clipboard";
import { followScrollLeft, followAllowed } from "../utils/followPlayhead";
import styles from "./ArrangementView.module.css";

interface ArrangementViewProps {
  projectMeta: SongMetadata;
  playing: boolean;
  /** Seek cursor position + seek request; owned by App (G29). */
  seekTick: number;
  onSeek: (tick: number) => void;
  /** Preview loop (transport-scratch, App-owned); drawn on the ruler. */
  previewLoop: import("../utils/previewLoop").PreviewLoopRange | null;
  loopEnabled: boolean;
  onPreviewLoopSet: (startTick: number, endTick: number) => void;
  onSelectRegions: (regions: SelectedRegion[]) => void;
  selectedRegions: SelectedRegion[];
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
  selectedInstrument: SelectedInstrument | null;
}

const M = DEFAULT_FM_MODULATOR;
const C = DEFAULT_FM_CARRIER;

// Fixed width of the track-header column; must match the
// `grid-template-columns: 180px 1fr` in ArrangementView.module.css so the
// follow-playhead logic knows the timeline's visible width.
const HEADER_COLUMN_WIDTH = 180;

// Shared with the view-state restore path (see types/model).
const channelType = channelTypeOf;

function channelLabel(track: Track): string {
  const ch = track.channel;
  if (ch === "PsgNoise") return "Noise";
  if (typeof ch === "object" && "Fm" in ch) return `FM${ch.Fm + 1}`;
  if (typeof ch === "object" && "Psg" in ch) return `PSG${ch.Psg + 1}`;
  if (typeof ch === "object" && "Dac" in ch) return "DAC";
  return "?";
}

export function ArrangementView({
  projectMeta,
  playing,
  seekTick,
  onSeek,
  previewLoop,
  loopEnabled,
  onPreviewLoopSet,
  onSelectRegions,
  selectedRegions,
  onSelectInstrument,
  selectedInstrument,
}: ArrangementViewProps) {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [showAddTrack, setShowAddTrack] = useState(false);
  // Arrangement snap (region create/move/resize, ruler loop drag). The
  // piano roll keeps its own grid selector.
  const [snapMode, setSnapMode] = useState<grid.SnapMode>("bar");
  const [channelLevels, setChannelLevels] = useState<number[]>([]);
  const [collapsedChannels, setCollapsedChannels] = useState<Set<string>>(new Set());
  const zoom = useArrangementZoom(projectMeta);
  const { interpolatedTick, currentTick } = usePlaybackPosition(playing, projectMeta.tempo, projectMeta.ticksPerBeat);
  const trackHeight = 60;
  // Manual scrolls (wheel, ruler drag) suspend follow-playhead briefly so the
  // user can inspect other bars during playback (G28).
  const lastManualScrollRef = useRef(-Infinity);

  useEffect(() => {
    // followAllowed also suppresses follow entirely while the preview loop
    // is active (owner ruling: looping playback must not move the view).
    if (!followAllowed(playing, loopEnabled, lastManualScrollRef.current, performance.now())) return;
    const body = zoom.bodyRef.current;
    if (!body) return;
    // Follow keys off the event-driven tick (interpolation only mutates a ref
    // between renders); paging accuracy at event granularity is plenty.
    const viewWidth = body.clientWidth - HEADER_COLUMN_WIDTH;
    const next = followScrollLeft(currentTick / zoom.ticksPerPixel, zoom.scrollLeft, viewWidth);
    if (next !== null) zoom.setScrollLeft(next);
  }, [currentTick, playing, loopEnabled, zoom.ticksPerPixel, zoom.scrollLeft, zoom.setScrollLeft, zoom.bodyRef]);

  const { visibleTracks, groupHeads } = useMemo(() => {
    const groups = new Map<string, Track[]>();
    for (const track of tracks) {
      const label = channelLabel(track);
      if (!groups.has(label)) groups.set(label, []);
      groups.get(label)!.push(track);
    }

    const visible: Track[] = [];
    const heads = new Map<string, { size: number; collapsed: boolean; label: string }>();

    for (const [label, groupTracks] of groups) {
      if (groupTracks.length > 1) {
        const collapsed = collapsedChannels.has(label);
        heads.set(groupTracks[0].id, { size: groupTracks.length, collapsed, label });
        if (collapsed) {
          visible.push(groupTracks[0]);
        } else {
          visible.push(...groupTracks);
        }
      } else {
        visible.push(groupTracks[0]);
      }
    }

    return { visibleTracks: visible, groupHeads: heads };
  }, [tracks, collapsedChannels]);

  const refresh = useCallback(async () => {
    const t = await ipc.listTracks();
    setTracks(t);
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  useEffect(() => {
    const interval = setInterval(refresh, 1000);
    return () => clearInterval(interval);
  }, [refresh]);

  // Undo/redo replaced the tracks server-side — re-fetch immediately.
  useEffect(() => {
    const onReverted = () => { refresh(); };
    window.addEventListener(SONG_REVERTED_EVENT, onReverted);
    return () => window.removeEventListener(SONG_REVERTED_EVENT, onReverted);
  }, [refresh]);

  useEffect(() => {
    if (!playing) {
      setChannelLevels([]);
      return;
    }
    const interval = setInterval(async () => {
      const state = await ipc.getPlaybackState();
      setChannelLevels(state.channelLevels);
    }, 60);
    return () => clearInterval(interval);
  }, [playing]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Never hijack keys while the user is typing in a field (G2).
      if (isEditableTarget(e.target)) return;
      if ((e.key === "Delete" || e.key === "Backspace") && selectedRegions.length > 0) {
        // Opening a region in the piano roll selects it, so while the roll
        // has a note selection Delete belongs to the notes — never cascade
        // into deleting the enclosing region (G1).
        if (pianoRollNoteSelectionActive()) return;
        e.preventDefault();
        // One gesture = one undo step: group the multi-delete batch.
        (async () => {
          await ipc.beginUndoGroup();
          try {
            await Promise.all(selectedRegions.map((r) => ipc.deleteRegion(r.trackId, r.regionId)));
          } finally {
            await ipc.endUndoGroup();
          }
          onSelectRegions([]);
          // Reload so the next playback stops sounding the deleted regions (G30).
          await ipc.reloadSequence();
          refresh();
        })();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "d" && selectedRegions.length > 0) {
        // G1 mirror: while the piano roll owns a note selection, Ctrl+D
        // keeps meaning duplicate-notes — never the enclosing region.
        if (pianoRollNoteSelectionActive()) return;
        e.preventDefault();
        (async () => {
          const newSel: SelectedRegion[] = [];
          // One gesture = one undo step: group the duplicate batch.
          await ipc.beginUndoGroup();
          try {
            for (const r of selectedRegions) {
              // Each duplicate lands immediately after its source.
              const newStart = r.startTick + r.durationTicks;
              const newId = await ipc.duplicateRegion(r.trackId, r.regionId, newStart);
              newSel.push({ ...r, regionId: newId, startTick: newStart });
            }
          } finally {
            await ipc.endUndoGroup();
          }
          await ipc.reloadSequence();
          await refresh();
          // The duplicates become the selection.
          onSelectRegions(newSel);
        })();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "c" && selectedRegions.length > 0) {
        // Same G1 scoping: a note selection owns copy too.
        if (pianoRollNoteSelectionActive()) return;
        e.preventDefault();
        // Snapshot ids + payload: the payload keeps paste working even if
        // the source regions are deleted before Ctrl+V.
        copyRegions(selectedRegions.flatMap((r) => {
          const track = tracks.find((t) => t.id === r.trackId);
          const region = track?.regions.find((rg) => rg.id === r.regionId);
          if (!region) return [];
          return [{
            trackId: r.trackId,
            regionId: r.regionId,
            payload: {
              startTick: region.startTick,
              durationTicks: region.durationTicks,
              notes: region.notes,
            },
          }];
        }));
      }
      // Paste arbitration: the module clipboard's lastCopiedKind() decides
      // whether Ctrl+V means notes (piano roll) or regions (here).
      if ((e.ctrlKey || e.metaKey) && e.key === "v" && lastCopiedKind() === "regions") {
        const clip = getRegionClipboard();
        if (clip.length === 0) return;
        e.preventDefault();
        // Paste anchor: the seek cursor, snapped DOWN to the bar (the same
        // bar snap regions use when dragged); the earliest copied region
        // lands there and the rest keep their relative offsets, each on the
        // track it was copied from.
        const oneBar = grid.ticksPerBar(projectMeta);
        const anchor = Math.floor(seekTick / oneBar) * oneBar;
        const base = Math.min(...clip.map((c) => c.payload.startTick));
        (async () => {
          const newSel: SelectedRegion[] = [];
          // One paste = one undo step: group the batch.
          await ipc.beginUndoGroup();
          try {
            for (const c of clip) {
              const newStart = anchor + (c.payload.startTick - base);
              let newId: string;
              try {
                // Server-side deep clone when the source still exists…
                newId = await ipc.duplicateRegion(c.trackId, c.regionId, newStart);
              } catch {
                // …else replay the copied payload (source was deleted).
                newId = await ipc.addRegion(c.trackId, newStart, c.payload.durationTicks);
                for (const n of c.payload.notes) {
                  // Replay lands on the SAME track the region was copied
                  // from, so per-note voices are always kind-safe to keep.
                  await ipc.addNote(c.trackId, newId, n.tick, n.pitch, n.velocity, n.durationTicks, n.instrumentId ?? null);
                }
              }
              const track = tracks.find((t) => t.id === c.trackId);
              newSel.push({
                trackId: c.trackId,
                trackName: track?.name ?? "",
                regionId: newId,
                channelType: track ? channelType(track) : "fm",
                startTick: newStart,
                durationTicks: c.payload.durationTicks,
              });
            }
          } finally {
            await ipc.endUndoGroup();
          }
          await ipc.reloadSequence();
          await refresh();
          // The pasted regions become the selection.
          onSelectRegions(newSel);
        })().catch((err) => {
          // Both paste branches can reject: duplicateRegion when the source
          // ids are gone (a clipboard from another project), and the
          // addRegion/addNote replay behind it. Unhandled, that surfaced as
          // an unhandled promise rejection. This view has no notice element
          // (the app has no toast system and the piano-roll header hint is
          // not reachable from here), so the console is the honest floor.
          console.error("Region paste failed:", err);
        });
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedRegions, onSelectRegions, refresh, tracks, seekTick, projectMeta]);

  function handleRegionDoubleClick(trackId: string, regionId: string) {
    const track = tracks.find((t) => t.id === trackId);
    if (!track) return;
    const region = track.regions.find((r) => r.id === regionId);
    if (!region) return;
    onSelectRegions([{
      trackId,
      trackName: track.name,
      regionId,
      channelType: channelType(track),
      startTick: region.startTick,
      durationTicks: region.durationTicks,
    }]);
  }

  async function handleRegionMove(srcTrackId: string, regionId: string, dstTrackId: string, startTick: number, tickDelta: number, trackDelta: number) {
    if (selectedRegions.length > 1 && selectedRegions.some((r) => r.regionId === regionId)) {
      // One gesture = one undo step: group the multi-region move batch.
      await ipc.beginUndoGroup();
      try {
        await Promise.all(selectedRegions.map((r) => {
          const srcIdx = visibleTracks.findIndex((t) => t.id === r.trackId);
          const dstIdx = Math.max(0, Math.min(visibleTracks.length - 1, srcIdx + trackDelta));
          const dst = visibleTracks[dstIdx].id;
          const newStart = Math.max(0, r.startTick + tickDelta);
          return ipc.moveRegion(r.trackId, r.regionId, dst, newStart);
        }));
      } finally {
        await ipc.endUndoGroup();
      }
    } else {
      // Single-region move commits once on mouseup — already one undo step.
      await ipc.moveRegion(srcTrackId, regionId, dstTrackId, startTick);
    }
    // Moves commit once per released drag (TimelineCanvas fires onRegionMove
    // from mouseup only), so a plain reload keeps playback audible-current
    // without needing a debounce (G30).
    await ipc.reloadSequence();
    refresh();
  }

  async function handleRegionResize(trackId: string, regionId: string, startTick: number, durationTicks: number) {
    await ipc.updateRegion(trackId, regionId, startTick, durationTicks);
    // Resizes also commit on mouse release only — one reload per gesture (G30).
    await ipc.reloadSequence();
    refresh();
  }

  /** Empty-lane double-click: create a one-bar region, open it in the piano
   *  roll (region selection drives BottomPanel), and reload the sequence so
   *  subsequent playback reflects it. */
  async function handleRegionCreate(trackIdx: number, startTick: number) {
    const track = visibleTracks[trackIdx];
    if (!track) return;
    const oneBar = grid.ticksPerBar(projectMeta);
    const regionId = await ipc.addRegion(track.id, startTick, oneBar);
    await refresh();
    onSelectRegions([{
      trackId: track.id,
      trackName: track.name,
      regionId,
      channelType: channelType(track),
      startTick,
      durationTicks: oneBar,
    }]);
    await ipc.reloadSequence();
  }

  // Seeks route through App, which owns the cursor and the transport call.
  const handleSeek = onSeek;

  function handleTrackClick(track: Track) {
    if (!track.instrumentId) return;
    const ct = channelType(track);
    onSelectInstrument({ type: ct, id: track.instrumentId });
  }

  async function addFm() {
    await ipc.addFmInstrument({
      id: "00000000-0000-0000-0000-000000000000",
      name: "New FM Patch",
      algorithm: 0,
      feedback: 0,
      operators: [M, M, M, C],
      metadata: { category: "", author: "", tags: [] },
    });
    refresh();
  }

  async function addPsg() {
    await ipc.addPsgInstrument({
      id: "00000000-0000-0000-0000-000000000000",
      name: "New PSG Envelope",
      volumeSequence: [15, 14, 13, 12, 10, 8, 6, 4, 2, 0],
      loopPoint: null,
      noiseMode: null,
      metadata: { category: "", author: "", tags: [] },
    });
    refresh();
  }

  async function addDac() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      filters: [
        { name: "Audio", extensions: ["wav"] },
        { name: "Raw PCM", extensions: ["pcm", "raw"] },
      ],
      title: "Import DAC Sample",
    });
    if (!selected) return;
    const path = selected as string;
    const ext = path.split(".").pop()?.toLowerCase();
    if (ext === "wav") {
      await ipc.importDacWav(path, 16000);
    } else {
      await ipc.importDacRaw(path, 16000);
    }
    refresh();
  }

  return (
    <div
      className={styles.arrangement}
      onWheel={(e) => {
        lastManualScrollRef.current = performance.now();
        zoom.handleWheel(e);
      }}
    >
      <div className={styles.rulerRow}>
        <div className={styles.headerSpacer} />
        <TimelineRuler
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          ticksPerBeat={projectMeta.ticksPerBeat}
          beatsPerBar={projectMeta.timeSignature[0]}
          loop={previewLoop}
          loopEnabled={loopEnabled}
          snapMode={snapMode}
          onSeek={handleSeek}
          onScrollChange={(v) => {
            lastManualScrollRef.current = performance.now();
            zoom.setScrollLeft(v);
          }}
          onLoopDrag={onPreviewLoopSet}
          onZoom={(anchorPx, factor) => {
            // Zoom moves the view too — suspend follow-playhead like any
            // other manual scroll (G28).
            lastManualScrollRef.current = performance.now();
            zoom.zoomAtBy(anchorPx, factor);
          }}
        />
      </div>
      <div className={styles.body} ref={zoom.bodyRef}>
        <div className={styles.headers}>
          {visibleTracks.map((track) => {
            const gh = groupHeads.get(track.id);
            return (
              <TrackHeader
                key={track.id}
                track={track}
                selected={selectedInstrument?.id === track.instrumentId}
                level={channelLevels[channelLevelIndex(track)] ?? 0}
                onUpdate={refresh}
                onClick={() => handleTrackClick(track)}
                isGroupHead={!!gh}
                groupCollapsed={gh?.collapsed}
                groupSize={gh?.size}
                onToggleCollapse={gh ? () => {
                  setCollapsedChannels(prev => {
                    const next = new Set(prev);
                    if (next.has(gh.label)) next.delete(gh.label);
                    else next.add(gh.label);
                    return next;
                  });
                } : undefined}
              />
            );
          })}
          <div className={styles.addButtons}>
            <label className={styles.snapControl} title="Arrangement snap (regions and loop drag)">
              Snap
              <select
                className={styles.snapSelect}
                value={snapMode}
                onChange={(e) => setSnapMode(e.target.value as grid.SnapMode)}
              >
                <option value="bar">Bar</option>
                <option value="beat">Beat</option>
                <option value="off">Off</option>
              </select>
            </label>
            <button className={styles.addBtn} onClick={() => setShowAddTrack(true)}>+ Track</button>
            <button className={styles.addBtn} onClick={addFm}>+ FM Patch</button>
            <button className={styles.addBtn} onClick={addPsg}>+ PSG Env</button>
            <button className={styles.addBtn} onClick={addDac}>+ DAC Sample</button>
          </div>
        </div>
        <TimelineCanvas
          tracks={visibleTracks}
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          trackHeight={trackHeight}
          ticksPerBeat={projectMeta.ticksPerBeat}
          beatsPerBar={projectMeta.timeSignature[0]}
          playbackTick={playing ? interpolatedTick : 0}
          playing={playing}
          selectedRegions={selectedRegions}
          onRegionClick={(trackId, regionId, ctrlKey) => {
            const track = visibleTracks.find((t) => t.id === trackId);
            if (!track) return;
            const region = track.regions.find((r) => r.id === regionId);
            if (!region) return;
            const sel: SelectedRegion = {
              trackId, trackName: track.name, regionId, channelType: channelType(track),
              startTick: region.startTick, durationTicks: region.durationTicks,
            };
            if (ctrlKey) {
              const exists = selectedRegions.some((r) => r.regionId === regionId);
              if (exists) {
                onSelectRegions(selectedRegions.filter((r) => r.regionId !== regionId));
              } else {
                onSelectRegions([...selectedRegions, sel]);
              }
            } else {
              onSelectRegions([sel]);
            }
          }}
          onRegionDoubleClick={handleRegionDoubleClick}
          onSelectRegions={onSelectRegions}
          onRegionMove={handleRegionMove}
          onRegionResize={handleRegionResize}
          onRegionCreate={handleRegionCreate}
          onSeek={handleSeek}
          seekTick={seekTick}
          snapMode={snapMode}
        />
      </div>
      {showAddTrack && (
        <AddTrackDialog
          driverId={projectMeta.driverId}
          onClose={() => setShowAddTrack(false)}
          onCreated={() => {
            setShowAddTrack(false);
            refresh();
          }}
        />
      )}
    </div>
  );
}
