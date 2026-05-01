import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export function TestPanel() {
  const [status, setStatus] = useState("Ready");

  async function playFmTone() {
    try {
      const result = await invoke<string>("play_fm_test_tone");
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function playPsgTone() {
    try {
      const result = await invoke<string>("play_psg_test_tone");
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function stopAll() {
    try {
      const result = await invoke<string>("stop_all_sound");
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  return (
    <div style={{ padding: "2rem", fontFamily: "monospace" }}>
      <h1>MegaDAW — Phase 1 Audio Test</h1>
      <p>Status: {status}</p>
      <div style={{ display: "flex", gap: "1rem", marginTop: "1rem" }}>
        <button onClick={playFmTone} style={buttonStyle}>
          Play FM Tone (YM2612)
        </button>
        <button onClick={playPsgTone} style={buttonStyle}>
          Play PSG Tone (SN76489)
        </button>
        <button onClick={stopAll} style={{ ...buttonStyle, background: "#c44" }}>
          Stop All
        </button>
      </div>
      <div style={{ marginTop: "2rem", color: "#888", fontSize: "0.85rem" }}>
        <p>FM: Plays a ~440Hz sine-ish tone through Nuked OPN2 emulation</p>
        <p>PSG: Plays a ~440Hz square wave through SN76489 emulation</p>
        <p>Stop: Resets both chips (panic)</p>
      </div>
    </div>
  );
}

const buttonStyle: React.CSSProperties = {
  padding: "0.75rem 1.5rem",
  fontSize: "1rem",
  fontFamily: "monospace",
  background: "#2a6",
  color: "white",
  border: "none",
  borderRadius: "4px",
  cursor: "pointer",
};
