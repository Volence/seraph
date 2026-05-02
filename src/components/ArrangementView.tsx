import { useState, useEffect, useCallback } from "react";
import type { Track, SongMetadata, SelectedRegion, SelectedInstrument, FmOperator } from "../types/model";
import { useArrangementZoom } from "../hooks/useArrangementZoom";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import { TrackHeader } from "./TrackHeader";
import { TimelineRuler } from "./TimelineRuler";
import { TimelineCanvas } from "./TimelineCanvas";
import * as ipc from "../api/ipc";
import styles from "./ArrangementView.module.css";

interface ArrangementViewProps {
  projectMeta: SongMetadata;
  playing: boolean;
  onSelectRegions: (regions: SelectedRegion[]) => void;
  selectedRegions: SelectedRegion[];
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
  selectedInstrument: SelectedInstrument | null;
}

const defaultOp: FmOperator = {
  detune: 0, multiple: 0, rateScale: 0, attackRate: 0,
  ampMod: false, d1r: 0, d2r: 0, sustainLevel: 0,
  releaseRate: 0, totalLevel: 127,
};

function channelType(track: Track): "fm" | "psg" | "dac" {
  const ch = track.channel;
  if (ch === "PsgNoise") return "psg";
  if (typeof ch === "object" && "Fm" in ch) return "fm";
  if (typeof ch === "object" && "Psg" in ch) return "psg";
  return "dac";
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
  const zoom = useArrangementZoom(projectMeta.ticksPerBeat);
  const { interpolatedTick } = usePlaybackPosition(playing, projectMeta.tempo, projectMeta.ticksPerBeat);
  const trackHeight = 60;

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

  async function handleRegionCreate(trackId: string, startTick: number, durationTicks: number) {
    await ipc.addRegion(trackId, startTick, durationTicks);
    refresh();
  }

  async function handleRegionMove(srcTrackId: string, regionId: string, dstTrackId: string, startTick: number, tickDelta: number, trackDelta: number) {
    if (selectedRegions.length > 1 && selectedRegions.some((r) => r.regionId === regionId)) {
      await Promise.all(selectedRegions.map((r) => {
        const srcIdx = tracks.findIndex((t) => t.id === r.trackId);
        const dstIdx = Math.max(0, Math.min(tracks.length - 1, srcIdx + trackDelta));
        const dst = tracks[dstIdx].id;
        const newStart = Math.max(0, r.startTick + tickDelta);
        return ipc.moveRegion(r.trackId, r.regionId, dst, newStart);
      }));
    } else {
      await ipc.moveRegion(srcTrackId, regionId, dstTrackId, startTick);
    }
    onSelectRegions([]);
    refresh();
  }

  async function handleRegionResize(trackId: string, regionId: string, startTick: number, durationTicks: number) {
    await ipc.updateRegion(trackId, regionId, startTick, durationTicks);
    refresh();
  }

  async function handleSeek(tick: number) {
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
      operators: [defaultOp, defaultOp, defaultOp, defaultOp],
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
        />
      </div>
      <div className={styles.body}>
        <div className={styles.headers}>
          {tracks.map((track) => (
            <TrackHeader
              key={track.id}
              track={track}
              selected={selectedInstrument?.id === track.instrumentId}
              onUpdate={refresh}
              onClick={() => handleTrackClick(track)}
            />
          ))}
          <div className={styles.addButtons}>
            <button className={styles.addBtn} onClick={addFm}>+ FM</button>
            <button className={styles.addBtn} onClick={addPsg}>+ PSG</button>
            <button className={styles.addBtn} onClick={addDac}>+ DAC</button>
          </div>
        </div>
        <TimelineCanvas
          tracks={tracks}
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          trackHeight={trackHeight}
          ticksPerBeat={projectMeta.ticksPerBeat}
          beatsPerBar={projectMeta.timeSignature[0]}
          playbackTick={playing ? interpolatedTick : 0}
          playing={playing}
          selectedRegions={selectedRegions}
          onRegionClick={(trackId, regionId, ctrlKey) => {
            const track = tracks.find((t) => t.id === trackId);
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
          onRegionCreate={handleRegionCreate}
          onRegionMove={handleRegionMove}
          onRegionResize={handleRegionResize}
        />
      </div>
    </div>
  );
}
