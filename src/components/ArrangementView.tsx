import { useState, useEffect, useCallback, useMemo } from "react";
import type { Track, SongMetadata, SelectedRegion, SelectedInstrument } from "../types/model";
import { DEFAULT_FM_MODULATOR, DEFAULT_FM_CARRIER } from "../types/model";
import { useArrangementZoom } from "../hooks/useArrangementZoom";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import { TrackHeader, channelLevelIndex } from "./TrackHeader";
import { TimelineRuler } from "./TimelineRuler";
import { TimelineCanvas } from "./TimelineCanvas";
import { AddTrackDialog } from "./AddTrackDialog";
import * as ipc from "../api/ipc";
import * as grid from "../utils/grid";
import styles from "./ArrangementView.module.css";

interface ArrangementViewProps {
  projectMeta: SongMetadata;
  playing: boolean;
  onSelectRegions: (regions: SelectedRegion[]) => void;
  selectedRegions: SelectedRegion[];
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
  selectedInstrument: SelectedInstrument | null;
}

const M = DEFAULT_FM_MODULATOR;
const C = DEFAULT_FM_CARRIER;

function channelType(track: Track): "fm" | "psg" | "dac" {
  const ch = track.channel;
  if (ch === "PsgNoise") return "psg";
  if (typeof ch === "object" && "Fm" in ch) return "fm";
  if (typeof ch === "object" && "Psg" in ch) return "psg";
  return "dac";
}

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
  onSelectRegions,
  selectedRegions,
  onSelectInstrument,
  selectedInstrument,
}: ArrangementViewProps) {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [showAddTrack, setShowAddTrack] = useState(false);
  const [channelLevels, setChannelLevels] = useState<number[]>([]);
  const [collapsedChannels, setCollapsedChannels] = useState<Set<string>>(new Set());
  const zoom = useArrangementZoom(projectMeta);
  const { interpolatedTick } = usePlaybackPosition(playing, projectMeta.tempo, projectMeta.ticksPerBeat);
  const [seekTick, setSeekTick] = useState(0);
  const trackHeight = 60;

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
      if ((e.key === "Delete" || e.key === "Backspace") && selectedRegions.length > 0) {
        e.preventDefault();
        Promise.all(selectedRegions.map((r) => ipc.deleteRegion(r.trackId, r.regionId))).then(() => {
          onSelectRegions([]);
          refresh();
        });
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedRegions, onSelectRegions, refresh]);

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
      await Promise.all(selectedRegions.map((r) => {
        const srcIdx = visibleTracks.findIndex((t) => t.id === r.trackId);
        const dstIdx = Math.max(0, Math.min(visibleTracks.length - 1, srcIdx + trackDelta));
        const dst = visibleTracks[dstIdx].id;
        const newStart = Math.max(0, r.startTick + tickDelta);
        return ipc.moveRegion(r.trackId, r.regionId, dst, newStart);
      }));
    } else {
      await ipc.moveRegion(srcTrackId, regionId, dstTrackId, startTick);
    }
    refresh();
  }

  async function handleRegionResize(trackId: string, regionId: string, startTick: number, durationTicks: number) {
    await ipc.updateRegion(trackId, regionId, startTick, durationTicks);
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

  async function handleSeek(tick: number) {
    setSeekTick(tick);
    await ipc.transportSeek(tick);
  }

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
    <div className={styles.arrangement} onWheel={zoom.handleWheel}>
      <div className={styles.rulerRow}>
        <div className={styles.headerSpacer} />
        <TimelineRuler
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          ticksPerBeat={projectMeta.ticksPerBeat}
          beatsPerBar={projectMeta.timeSignature[0]}
          onSeek={handleSeek}
          onScrollChange={zoom.setScrollLeft}
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
