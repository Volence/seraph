import { useState, useEffect, useCallback } from "react";
import type { Track, SongMetadata, SelectedRegion } from "../types/model";
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
  onSelectRegion: (region: SelectedRegion | null) => void;
  selectedRegion: SelectedRegion | null;
}

export function ArrangementView({ projectMeta, playing, onSelectRegion, selectedRegion }: ArrangementViewProps) {
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

  function handleRegionDoubleClick(trackId: string, regionId: string) {
    const track = tracks.find((t) => t.id === trackId);
    if (!track) return;
    const region = track.regions.find((r) => r.id === regionId);
    if (!region) return;
    const ch = track.channel;
    const ct = ch === "PsgNoise" ? "psg" as const :
               typeof ch === "object" && "Fm" in ch ? "fm" as const :
               typeof ch === "object" && "Psg" in ch ? "psg" as const : "dac" as const;
    onSelectRegion({
      trackId,
      trackName: track.name,
      regionId,
      channelType: ct,
      startTick: region.startTick,
      durationTicks: region.durationTicks,
    });
  }

  async function handleCreateRegion(trackId: string, startTick: number) {
    const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
    const snapped = Math.floor(startTick / ticksPerBar) * ticksPerBar;
    await ipc.addRegion(trackId, snapped, ticksPerBar);
    refresh();
  }

  async function handleSeek(tick: number) {
    await ipc.transportSeek(tick);
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
              onUpdate={refresh}
            />
          ))}
        </div>
        <TimelineCanvas
          tracks={tracks}
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          trackHeight={trackHeight}
          playbackTick={playing ? interpolatedTick : 0}
          playing={playing}
          selectedRegion={selectedRegion}
          onRegionClick={(trackId, regionId) => {
            const track = tracks.find((t) => t.id === trackId);
            if (!track) return;
            const region = track.regions.find((r) => r.id === regionId);
            if (!region) return;
            const ch2 = track.channel;
            const ct = ch2 === "PsgNoise" ? "psg" as const :
                       typeof ch2 === "object" && "Fm" in ch2 ? "fm" as const :
                       typeof ch2 === "object" && "Psg" in ch2 ? "psg" as const : "dac" as const;
            onSelectRegion({
              trackId, trackName: track.name, regionId, channelType: ct,
              startTick: region.startTick, durationTicks: region.durationTicks,
            });
          }}
          onRegionDoubleClick={handleRegionDoubleClick}
          onEmptyDoubleClick={handleCreateRegion}
        />
      </div>
    </div>
  );
}
