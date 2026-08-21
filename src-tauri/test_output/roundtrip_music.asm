; ============================================================
; Song: Snd AIZ1 Header
; Exported from Seraph
; ============================================================

Snd_AIZ1_Header_Header:
	smpsHeaderStartSong 3
	smpsHeaderVoice		Snd_AIZ1_Header_Voices
	smpsHeaderChan		$06, $03
	smpsHeaderTempo		$01, $1F
	smpsHeaderDAC		Snd_AIZ1_Header_DAC1, $00, $00
	smpsHeaderFM		Snd_AIZ1_Header_FM1, $00, $00
	smpsHeaderFM		Snd_AIZ1_Header_FM2, $00, $00
	smpsHeaderFM		Snd_AIZ1_Header_FM3, $00, $00
	smpsHeaderFM		Snd_AIZ1_Header_FM4, $00, $00
	smpsHeaderFM		Snd_AIZ1_Header_FM5, $00, $00
	smpsHeaderPSG		Snd_AIZ1_Header_PSG1, $00, $00, $00, sTone_0C
	smpsHeaderPSG		Snd_AIZ1_Header_PSG2, $00, $00, $00, sTone_0C
	smpsHeaderPSG		Snd_AIZ1_Header_PSG_Noise4, $00, $00, $00, sTone_0C

; ------------------------------------------------------------
; DAC Channel 1 - "DAC"
; ------------------------------------------------------------
Snd_AIZ1_Header_DAC1:
	dc.b dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dSnareS3, dKickS3, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06
	dc.b dMetalHit, dSnareS3, dSnareS3, dSnareS3, dSnareS3, dKickS3, dHighTom, dMidTomS3, dLowTomS3, dKickS3, dHigherMetalHit, $09
	dc.b dHigherMetalHit, $02, dHigherMetalHit, $01, dHigherMetalHit, $06, dHigherMetalHit, dHigherMetalHit, $12, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06
	dc.b dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $18
	dc.b dKickS3, $17, dSnareS3, $01, dSnareS3, $06, dSnareS3, $0C, dSnareS3, dSnareS3, $06, dSnareS3, $0C
	dc.b dKickS3, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dSnareS3, dKickS3
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, dHigherMetalHit, dHighMetalHit, dLowTomS3, dKickS3, $0C, dSnareS3, $06, dSnareS3
	dc.b dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dSnareS3, dKickS3, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dSnareS3, dSnareS3, dSnareS3, $0C, dSnareS3, $06
	dc.b dSnareS3, dSnareS3, $0C, dKickS3, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit
	dc.b dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dHighTom, $06
	dc.b dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3
	dc.b dSnareS3, dHighMetalHit, dSnareS3, dSnareS3, $0C, dSnareS3, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C
	dc.b dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C
	dc.b dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dSnareS3, $06
	dc.b dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C
	dc.b dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06
	dc.b dLowTomS3, dKickS3, $0C, dMidTomS3, $06, dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C
	dc.b dHighTom, $06, dMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dKickS3, $0C, dMidTomS3, $06
	dc.b dLowMetalHit, dKickS3, $0C, dHighMetalHit, $06, dLowTomS3, dSnareS3, $0C, dHighTom, $06, dMetalHit, $03
	dc.b dMidMetalHit, $02, dMidMetalHit, $01, dHigherMetalHit, $06, dHigherMetalHit, $0C, dHigherMetalHit, dLowTomS3, $06, dSnareS3
	dc.b dKickS3, $0C, dKickS3, dKickS3, $03, dKickS3, dSnareS3, $06, nRst, $12
	smpsStop

; ------------------------------------------------------------
; FM Channel 1 - "FM1"
; ------------------------------------------------------------
Snd_AIZ1_Header_FM1:
	smpsSetvoice	$00
	smpsModSet		$03, $01, $02, $05
	smpsFMAlterVol	$0F
	dc.b nC3, $0B, nRst, $02
	smpsSetvoice	$01
	dc.b nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05, nRst, $01
	dc.b nBb3, $05, nRst, $07, nA3, $05, nRst, $01, nBb3, $05, nRst, $07
	dc.b nBb3, $05, nRst, $07, nC4, $0B, nRst, $01
	smpsSetvoice	$00
	dc.b nC3, $0B, nRst, $01
	smpsSetvoice	$01
	dc.b nBb3, $01, nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05
	dc.b nRst, $01, nBb3, $05, nRst, $07
	smpsSetvoice	$00
	dc.b nBb2, $05, nRst, $07, nBb2, $05, nRst, $07, nBb2, $05, nRst, $01
	dc.b nB2, $0B, nRst, $01, nC3, $0B, nRst, $01
	smpsSetvoice	$01
	dc.b nB3, $01, nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05
	dc.b nRst, $01, nBb3, $05, nRst, $07, nA3, $05, nRst, $01, nBb3, $05
	dc.b nRst, $07, nBb3, $05, nRst, $08, nC4, $0A, nRst, $01
	smpsSetvoice	$00
	dc.b nC3, $0B, nRst, $01
	smpsSetvoice	$01
	dc.b nC4, $05, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05, nRst, $01
	dc.b nBb3, $05, nRst, $07
	smpsSetvoice	$00
	dc.b nBb2, $05, nRst, $07, nBb2, $05, nRst, $07, nBb2, $05, nRst, $01
	dc.b nB2, $0B, nRst, $01, nC3, $0B, nRst, $02
	smpsSetvoice	$01
	dc.b nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05, nRst, $01
	dc.b nBb3, $05, nRst, $07, nA3, $05, nRst, $01, nBb3, $05, nRst, $07
	dc.b nBb3, $05, nRst, $07, nC4, $0B, nRst, $01
	smpsSetvoice	$00
	dc.b nC3, $0B, nRst, $01
	smpsSetvoice	$01
	dc.b nBb3, $01, nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05
	dc.b nRst, $01, nBb3, $05, nRst, $07
	smpsSetvoice	$00
	dc.b nBb2, $05, nRst, $07, nBb2, $05, nRst, $07, nBb2, $05, nRst, $01
	dc.b nB2, $0B, nRst, $01, nC3, $0B, nRst, $01
	smpsSetvoice	$01
	dc.b nB3, $01, nC4, $04, nRst, $07, nBb3, $0B, nRst, $01, nA3, $05
	dc.b nRst, $01, nBb3, $05, nRst, $07, nA3, $05, nRst, $01, nBb3, $05
	dc.b nRst, $07, nBb3, $05, nRst, $08, nC4, $0A, nRst, $01
	smpsSetvoice	$00
	dc.b nG2, $05, nRst, $01, nG2, $05, nRst, $01, nG2, $05, nRst, $01
	dc.b nG2, $05, nRst, $1F, nF2, $05, nRst, $01, nA2, $0B, nRst, $01
	dc.b nBb2, $0B, nRst, $01, nB2, $0B, nRst, $01, nC3, $16, nRst, $02
	dc.b nC3, $10, nRst, $02, nC3, $10, nRst, $02, nG2, $0A, nRst, $02
	dc.b nC3, $0A, nRst, $02, nG2, $0A, nRst, $02, nF2, $16, nRst, $02
	dc.b nF2, $10, nRst, $02, nF2, $0A, nRst, $02, nF2, $04, nRst, $02
	dc.b nA2, $0A, nRst, $02, nBb2, $0A, nRst, $02, nB2, $0A, nRst, $02
	dc.b nC3, $16, nRst, $02, nC3, $10, nRst, $02, nC3, $10, nRst, $02
	dc.b nG2, $0A, nRst, $02, nC3, $0A, nRst, $02, nG2, $0A, nRst, $02
	dc.b nF2, $16, nRst, $02, nF2, $10, nRst, $02, nF2, $0A, nRst, $02
	dc.b nF2, $04, nRst, $02, nA2, $0A, nRst, $02, nBb2, $0A, nRst, $02
	dc.b nB2, $0A, nRst, $02, nC3, $16, nRst, $02, nC3, $10, nRst, $02
	dc.b nC3, $10, nRst, $02, nG2, $0A, nRst, $02, nC3, $0A, nRst, $02
	dc.b nG2, $0A, nRst, $02, nF2, $16, nRst, $02, nF2, $10, nRst, $02
	dc.b nF2, $0A, nRst, $02, nF2, $04, nRst, $02, nA2, $0A, nRst, $02
	dc.b nBb2, $0A, nRst, $02, nB2, $0A, nRst, $02, nC3, $16, nRst, $02
	dc.b nC3, $10, nRst, $02, nC3, $10, nRst, $02, nG2, $0A, nRst, $02
	dc.b nC3, $0A, nRst, $02, nG2, $0A, nRst, $02, nF2, $16, nRst, $02
	dc.b nF2, $10, nRst, $02, nF2, $0A, nRst, $02, nF2, $04, nRst, $02
	dc.b nA2, $0A, nRst, $02, nBb2, $0A, nRst, $02, nB2, $0A, nRst, $02
	dc.b nC3, $16, nRst, $02, nC3, $10, nRst, $02, nC3, $10, nRst, $02
	dc.b nG2, $0A, nRst, $02, nC3, $0A, nRst, $02, nG2, $0A, nRst, $02
	dc.b nF2, $16, nRst, $02, nF2, $10, nRst, $02, nF2, $0A, nRst, $02
	dc.b nF2, $04, nRst, $02, nA2, $0A, nRst, $02, nBb2, $0A, nRst, $02
	dc.b nB2, $0A, nRst, $02, nC3, $16, nRst, $02, nC3, $10, nRst, $02
	dc.b nC3, $10, nRst, $02, nG2, $0A, nRst, $02, nC3, $0A, nRst, $02
	dc.b nG2, $0A, nRst, $02, nF2, $16, nRst, $02, nF2, $10, nRst, $02
	dc.b nF2, $0A, nRst, $02, nF2, $04, nRst, $02, nA2, $0A, nRst, $02
	dc.b nBb2, $0A, nRst, $02, nB2, $0A, nRst, $02, nC3, $16, nRst, $02
	dc.b nC3, $10, nRst, $02, nC3, $10, nRst, $02, nG2, $0A, nRst, $02
	dc.b nC3, $0A, nRst, $02, nG2, $0A, nRst, $02, nF2, $16, nRst, $02
	dc.b nF2, $10, nRst, $02, nF2, $0A, nRst, $02, nF2, $04, nRst, $02
	dc.b nA2, $0A, nRst, $02, nBb2, $0A, nRst, $02, nB2, $0A, nRst, $02
	dc.b nC3, $16, nRst, $02, nC3, $10, nRst, $02, nC3, $10, nRst, $02
	dc.b nG2, $0A, nRst, $02, nC3, $0A, nRst, $02, nG2, $0A, nRst, $02
	dc.b nF2, $16, nRst, $02, nF2, $10, nRst, $02, nF2, $0A, nRst, $02
	dc.b nF2, $04, nRst, $02, nC3, $04, nRst, $02, nF3, $04, nRst, $02
	dc.b nF3, $04, nRst, $02, nC3, $04, nRst, $02, nF2, $0A, nRst, $02
	dc.b nE2, $16, nRst, $02, nE2, $10, nRst, $02, nA2, $10, nRst, $02
	dc.b nA2, $0A, nRst, $02, nE3, $0A, nRst, $02, nA2, $0A, nRst, $02
	dc.b nD3, $16, nRst, $02, nD3, $10, nRst, $02, nG2, $0A, nRst, $02
	dc.b nG2, $04, nRst, $02, nG3, $0A, nRst, $02, nG3, $0A, nRst, $02
	dc.b nF3, $0A, nRst, $02, nE3, $16, nRst, $02, nE3, $10, nRst, $02
	dc.b nA2, $10, nRst, $02, nA2, $0A, nRst, $02, nE3, $0A, nRst, $02
	dc.b nA2, $0A, nRst, $02, nD3, $16, nRst, $02, nD3, $10, nRst, $02
	dc.b nG2, $0A, nRst, $02, nG2, $04, nRst, $02, nG3, $0A, nRst, $02
	dc.b nG3, $0A, nRst, $02, nF3, $0A, nRst, $02, nE3, $16, nRst, $02
	dc.b nE3, $10, nRst, $02, nA2, $10, nRst, $02, nA2, $0A, nRst, $02
	dc.b nE3, $0A, nRst, $02, nA2, $0A, nRst, $02, nD3, $16, nRst, $02
	dc.b nD3, $10, nRst, $02, nG2, $0A, nRst, $02, nG2, $04, nRst, $02
	dc.b nG3, $0A, nRst, $02, nG3, $0A, nRst, $02, nF3, $0A, nRst, $02
	dc.b nE3, $16, nRst, $02, nE3, $10, nRst, $02, nA2, $10, nRst, $02
	dc.b nA2, $0A, nRst, $02, nE3, $0A, nRst, $02, nA2, $0A, nRst, $02
	dc.b nD3, $16, nRst, $02, nD3, $10, nRst, $02, nG2, $0A, nRst, $02
	dc.b nG2, $04, nRst, $02, nG3, $0A, nRst, $02, nFs3, $0A, nRst, $02
	dc.b nFs3, $0A, nRst, $02, nF3, $16, nRst, $02, nF3, $10, nRst, $02
	dc.b nC3, $10, nRst, $02, nC3, $0A, nRst, $02, nF3, $0A, nRst, $02
	dc.b nF3, $0A, nRst, $02, nD3, $16, nRst, $02, nD3, $10, nRst, $02
	dc.b nA2, $10, nRst, $02, nA2, $0A, nRst, $02, nD3, $0A, nRst, $02
	dc.b nD3, $0A, nRst, $02, nBb2, $16, nRst, $02, nBb2, $10, nRst, $02
	dc.b nF2, $10, nRst, $02, nF2, $0A, nRst, $02, nBb2, $0A, nRst, $02
	dc.b nBb2, $0A, nRst, $02, nG2, $16, nRst, $02, nG2, $10, nRst, $02
	dc.b nB2, $10, nRst, $02, nB2, $0A, nRst, $02, nC3, $0A, nRst, $02
	dc.b nD3, $0A, nRst, $02
	smpsStop

; ------------------------------------------------------------
; FM Channel 2 - "FM2"
; ------------------------------------------------------------
Snd_AIZ1_Header_FM2:
	smpsSetvoice	$02
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	smpsFMAlterVol	$16
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$06
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FA
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FC
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$06
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$F8
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$04
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4, nBb4, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$04
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$FE
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$02
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsAlterNote	$05
	dc.b nC5, $06, nC5, nC6, nC5, nBb5, nC5, nBb5, nC6
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	dc.b nD4, $06, nF4, nD5, nC5, $1E
	smpsSetvoice	$02
	smpsModSet		$0F, $01, $06, $05
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$06
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FA
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FC
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$06
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$F8
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$04
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4, nBb4, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$04
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$FE
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$02
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	dc.b nA4, $06, nA4, nA4, nA4, nRst, $24
	smpsSetvoice	$05
	smpsModSet		$0F, $01, $06, $05
	smpsPan			panCenter, $00
	smpsFMAlterVol	$F5
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nD3, $01, nC4, nD4, $39, nRst, $55
	smpsSetvoice	$06
	smpsAlterNote	$03
	smpsPan			panRight, $00
	smpsFMAlterVol	$03
	dc.b nE5, $05, nRst, $01, nF5, $05, nRst, $01, nG5, $05, nRst, $0D
	dc.b nC5, $05, nRst, $0D, nBb5, $11, nRst, $07, nBb5, $05, nRst, $07
	dc.b nBb5, $05, nRst, $07, nG5, $05, nRst, $07, nA5, $05, nRst, $0D
	dc.b nF5, $05, nRst, $0D, nC5, $29, nRst, $07, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $11, nRst, $07, nBb5, $05, nRst, $07, nBb5, $05, nRst, $07
	dc.b nC6, $05, nRst, $07, nA5, $2F, nRst, $25, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $0B, nRst, $07, nBb5, $02, nRst, $04, nBb5, $02, nRst, $0A
	dc.b nBb5, $05, nRst, $07, nG5, $05, nRst, $07, nA5, $05, nRst, $0D
	dc.b nF5, $05, nRst, $0D, nC5, $23, nRst, $0D, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $11, nRst, $07, nBb5, $05, nRst, $07, nBb5, $05, nRst, $07
	dc.b nC6, $05, nRst, $07, nA5, $11, nRst, $01, nBb5, $11, nRst, $01
	dc.b nC6, $23, nRst, $1F
	smpsSetvoice	$05
	smpsPan			panCenter, $00
	smpsFMAlterVol	$08
	dc.b nG2, $01, nF3, nG3, $2D, nRst, $01, nE2, nD3, nE3, $2D, nRst, $01
	dc.b nA2, nG3, nA3, $0D, nRst, $01, nG2, nF3, nG3, $0D, nRst, $01
	dc.b nF2, nEb3, nF3, $0D, nRst, $01, nE2, nD3, nE3, $0D, nRst, $01
	dc.b nF2, nEb3, nF3, $0D, nRst, $01, nA2, nG3, nA3, $0D, nRst, $07
	smpsSetvoice	$06
	smpsPan			panRight, $00
	smpsFMAlterVol	$FE
	dc.b nG4, $0B, nRst, $01, nD5, $0B, nRst, $01, nG5, $0B, nRst, $01
	dc.b nF5, $05, nRst, $0D, nE5, $05, nRst, $0D, nC5, $05, nRst, $07
	dc.b nA4, $30, nD5, $05, nRst, $0D, nC5, $05, nRst, $0D, nB4, $05
	dc.b nRst, $0D
	smpsSetvoice	$05
	smpsPan			panCenter, $00
	smpsFMAlterVol	$02
	dc.b nG3, $30, nE3, $2F, nRst, $01, nA3, $0F, nRst, $01, nG3, $0F
	dc.b nRst, $01, nF3, $0F, nRst, $01, nE3, $0F, nRst, $01, nF3, $0F
	dc.b nRst, $01, nA3, $0F, nRst, $07
	smpsSetvoice	$06
	smpsPan			panRight, $00
	smpsFMAlterVol	$FE
	dc.b nG4, $0B, nRst, $01, nD5, $0B, nRst, $01, nG5, $0B, nRst, $01
	dc.b nF5, $05, nRst, $0D, nE5, $05, nRst, $0D, nC5, $05, nRst, $07
	dc.b nA4, $30, nD5, $05, nRst, $0D, nC5, $05, nRst, $0D, nB4, $05
	dc.b nRst, $07
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nA4, $1D, nRst, $07, nA4, $02, nRst, $04, nB4, $02, nRst, $04
	dc.b nC5, $12, nB4, nA4, $0B, nRst, $01, nC5, $1D, nRst, $07, nC5, $02
	dc.b nRst, $04, nD5, $02, nRst, $04, nE5, $12, nD5, nC5, $0B, nRst, $01
	dc.b nD5, $30, nA4, nC5, $18, nB4, nC5, nD5
	smpsStop

; ------------------------------------------------------------
; FM Channel 3 - "FM3"
; ------------------------------------------------------------
Snd_AIZ1_Header_FM3:
	smpsSetvoice	$02
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	smpsFMAlterVol	$16
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$06
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FA
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FC
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$06
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$F8
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$04
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4, nBb4, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$04
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$FE
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$02
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsAlterNote	$FB
	dc.b nC5, $06, nC5, nC6, nC5, nBb5, nC5, nBb5, nC6
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	dc.b nF4, $06, nBb4, nF5, nE5, $1E
	smpsSetvoice	$02
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$05
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$06
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FA
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FC
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$06
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$F8
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$04
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$02
	dc.b nBb4, $04, nG4, nBb4, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04, nG4
	smpsFMAlterVol	$FE
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$04
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$02
	dc.b nA4, $04, nF4
	smpsFMAlterVol	$FE
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nBb4, $04
	smpsFMAlterVol	$02
	dc.b nA4, $04
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$02
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nE4, nG4, nE4
	smpsFMAlterVol	$FE
	dc.b nG4, $04, nC6, $06, nC6, nC6, nC6, nRst, $27
	smpsSetvoice	$05
	smpsAlterNote	$03
	smpsPan			panCenter, $00
	smpsFMAlterVol	$FA
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nD4, $3B, nRst, $3D, nF2, $01, nEb3
	dc.b nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01, nEb4, nF4, $0A
	dc.b nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01, nBb3, nC4, $03
	dc.b nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D, nF2, $01, nEb3
	dc.b nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01, nEb4, nF4, $0A
	dc.b nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01, nBb3, nC4, $03
	dc.b nRst, $0D, nD3, $01, nC4, nD4, $39, nRst, $52
	smpsSetvoice	$06
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	smpsFMAlterVol	$06
	dc.b nE5, $05, nRst, $01, nF5, $05, nRst, $01, nG5, $05, nRst, $0D
	dc.b nC5, $05, nRst, $0D, nBb5, $11, nRst, $07, nBb5, $05, nRst, $07
	dc.b nBb5, $05, nRst, $07, nG5, $05, nRst, $07, nA5, $05, nRst, $0D
	dc.b nF5, $05, nRst, $0D, nC5, $29, nRst, $07, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $11, nRst, $07, nBb5, $05, nRst, $07, nBb5, $05, nRst, $07
	dc.b nC6, $05, nRst, $07, nA5, $2F, nRst, $25, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $0B, nRst, $07, nBb5, $02, nRst, $04, nBb5, $02, nRst, $0A
	dc.b nBb5, $05, nRst, $07, nG5, $05, nRst, $07, nA5, $05, nRst, $0D
	dc.b nF5, $05, nRst, $0D, nC5, $23, nRst, $0D, nE5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nG5, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nBb5, $11, nRst, $07, nBb5, $05, nRst, $07, nBb5, $05, nRst, $07
	dc.b nC6, $05, nRst, $07, nA5, $11, nRst, $01, nBb5, $11, nRst, $01
	dc.b nC6, $23, nRst, $19
	smpsSetvoice	$05
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$03
	smpsPan			panCenter, $00
	dc.b nG2, $01, nF3, nG3, $2D, nRst, $01, nE2, nD3, nE3, $2D, nRst, $01
	dc.b nA2, nG3, nA3, $0D, nRst, $01, nG2, nF3, nG3, $0D, nRst, $01
	dc.b nF2, nEb3, nF3, $0D, nRst, $01, nE2, nD3, nE3, $0D, nRst, $01
	dc.b nF2, nEb3, nF3, $0D, nRst, $01, nA2, nG3, nA3, $0D, nRst, $01
	dc.b nG3, $5F, nRst, $07, nA4, $2F, nRst, $01, nD5, $05, nRst, $0D
	dc.b nC5, $05, nRst, $0D, nB4, $05, nRst, $01, nG3, $2F, nRst, $01
	dc.b nE3, $2F, nRst, $01, nA3, $0F, nRst, $01, nG3, $0F, nRst, $01
	dc.b nF3, $0F, nRst, $01, nE3, $0F, nRst, $01, nF3, $0F, nRst, $01
	dc.b nA3, $0F, nRst, $01, nG3, $5F, nRst, $01, nA4, $2F, nRst, $01
	dc.b nD5, $05, nRst, $0D, nC5, $05, nRst, $0D, nB4, $05, nRst, $67
	smpsSetvoice	$06
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nF4, $1D, nRst, $07, nF4, $02, nRst, $04, nG4, $02, nRst, $04
	dc.b nA4, $11, nRst, $01, nG4, $11, nRst, $01, nF4, $0B, nRst, $01
	smpsSetvoice	$04
	dc.b nA4, $2F, nRst, $01, nF4, $2F, nRst, $01, nA4, $17, nRst, $01
	dc.b nG4, $17, nRst, $01, nA4, $17, nRst, $01, nB4, $17, nRst, $01
	smpsStop

; ------------------------------------------------------------
; FM Channel 4 - "FM4"
; ------------------------------------------------------------
Snd_AIZ1_Header_FM4:
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	smpsFMAlterVol	$16
	dc.b nG4, $05, nRst, $0D, nG4, $05, nRst, $19, nG4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $19, nF4, $05, nRst, $0D, nF4, $05, nRst, $07
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nD4, $04, nRst, $02, nE4, $04, nRst, $02, nF4, $04, nRst, $08
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nF4, $05, nRst, $07, nF4, $05, nRst, $07, nF4, $05, nRst, $01
	dc.b nFs4, $05, nRst, $07, nG4, $05, nRst, $0D, nG4, $05, nRst, $19
	dc.b nG4, $05, nRst, $0D, nG4, $05, nRst, $19, nF4, $05, nRst, $0D
	dc.b nF4, $05, nRst, $19
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nF4, $04, nRst, $02, nBb4, $04, nRst, $02, nF5, $04, nRst, $02
	dc.b nE5, $1D, nRst, $01
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nG4, $05, nRst, $0D, nG4, $05, nRst, $19, nG4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $19, nF4, $05, nRst, $0D, nF4, $05, nRst, $07
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nD4, $04, nRst, $02, nE4, $04, nRst, $02, nF4, $04, nRst, $08
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nF4, $05, nRst, $07, nF4, $05, nRst, $07, nF4, $05, nRst, $01
	dc.b nFs4, $05, nRst, $07, nG4, $05, nRst, $0D, nG4, $05, nRst, $19
	dc.b nG4, $05, nRst, $0D, nG4, $05, nRst, $19
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nC5, $05, nRst, $01, nC5, $05, nRst, $01, nC5, $05, nRst, $01
	dc.b nC5, $05, nRst, $55
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsAlterNote	$FB
	dc.b nD4, $02, nRst, $0A, nE4, $02, nRst, $16, nD4, $11, nRst, $01
	dc.b nE4, $02, nRst, $28, nA5, $05, nRst, $01, nBb5, $05, nRst, $07
	dc.b nBb5, $02, nRst, $04, nBb5, $02, nRst, $04, nBb5, $02, nRst, $04
	dc.b nBb5, $02, nRst, $0A, nA5, $23, nRst, $0D, nD4, $02, nRst, $0A
	dc.b nE4, $02, nRst, $16, nD4, $11, nRst, $01, nE4, $02, nRst, $28
	dc.b nA5, $05, nRst, $01, nBb5, $05, nRst, $07, nBb5, $02, nRst, $04
	dc.b nBb5, $02, nRst, $04, nBb5, $02, nRst, $04, nBb5, $02, nRst, $0A
	dc.b nC6, $05, nRst, $01, nBb5, $1D, nRst, $0D, nD4, $02, nRst, $0A
	dc.b nE4, $02, nRst, $16, nD4, $11, nRst, $01, nE4, $02, nRst, $28
	dc.b nA5, $05, nRst, $01, nBb5, $05, nRst, $07, nBb5, $02, nRst, $04
	dc.b nBb5, $02, nRst, $04, nBb5, $02, nRst, $04, nBb5, $02, nRst, $0A
	dc.b nA5, $23, nRst, $0D, nD4, $02, nRst, $0A, nE4, $02, nRst, $16
	dc.b nD4, $11, nRst, $01, nE4, $02, nRst, $28, nA5, $05, nRst, $01
	dc.b nBb5, $05, nRst, $07, nBb5, $02, nRst, $04, nBb5, $02, nRst, $04
	dc.b nBb5, $02, nRst, $04, nBb5, $02, nRst, $0A, nC6, $05, nRst, $01
	dc.b nBb5, $1D, nRst, $0D
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsPan			panRight, $00
	dc.b nE4, $0B, nRst, $01, nE4, $0B, nRst, $0D, nD4, $05, nRst, $0D
	dc.b nE4, $05, nRst, $19, nC3, $05, nRst, $01, nG3, $05, nRst, $01
	dc.b nC4, $05, nRst, $01, nBb3, $05, nRst, $07, nBb3, $05, nRst, $07
	dc.b nA3, $05, nRst, $07, nA3, $05, nRst, $07, nF3, $05, nRst, $01
	dc.b nC3, $05, nRst, $1F, nE4, $0B, nRst, $01, nE4, $0B, nRst, $0D
	dc.b nD4, $05, nRst, $0D, nE4, $05, nRst, $19, nC3, $05, nRst, $01
	dc.b nG3, $05, nRst, $01, nC4, $05, nRst, $01, nBb3, $05, nRst, $07
	dc.b nBb3, $05, nRst, $07, nA3, $05, nRst, $07, nA3, $05, nRst, $07
	dc.b nBb3, $05, nRst, $01, nC4, $05, nRst, $1F, nE4, $0B, nRst, $01
	dc.b nE4, $0B, nRst, $0D, nD4, $05, nRst, $0D, nE4, $05, nRst, $19
	dc.b nC3, $05, nRst, $01, nG3, $05, nRst, $01, nC4, $05, nRst, $01
	dc.b nBb3, $05, nRst, $07, nBb3, $05, nRst, $07, nA3, $05, nRst, $07
	dc.b nA3, $05, nRst, $07, nF3, $05, nRst, $01, nC3, $05, nRst, $1F
	dc.b nE4, $0B, nRst, $01, nE4, $0B, nRst, $0D, nD4, $05, nRst, $0D
	dc.b nE4, $05, nRst, $19, nC3, $05, nRst, $01, nG3, $05, nRst, $01
	dc.b nC4, $05, nRst, $01, nBb3, $05, nRst, $07, nBb3, $05, nRst, $07
	dc.b nA3, $05, nRst, $07
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $06, $05
	smpsAlterNote	$FE
	smpsPan			panLeft, $00
	dc.b nA4, $02, nRst, $04, nBb4, $02, nRst, $04, nC5, $02, nRst, $04
	dc.b nEb5, $02, nRst, $04, nD5, $02, nRst, $04, nBb4, $02, nRst, $04
	dc.b nC5, $02, nRst, $10
	smpsSetvoice	$02
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$05
	dc.b nG4, $0B, nRst, $01, nC5, $0B, nRst, $01, nG5, $0B, nRst, $01
	dc.b nF5, $05, nRst, $0D, nE5, $05, nRst, $0D, nC5, $05, nRst, $07
	dc.b nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$F8
	dc.b nC5, $05, nRst, $07, nB4, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nD5, $05, nRst, $07, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01
	smpsFMAlterVol	$F4
	dc.b nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$F8
	dc.b nA4, $03, nRst, $09, nF5, $05, nRst, $0D, nE5, $05, nRst, $0D
	dc.b nD5, $05, nRst, $07, nB4, $05, nRst, $0D, nA4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $13, nG4, $0B, nRst, $01, nC5, $0B, nRst, $01
	dc.b nG5, $0B, nRst, $01, nF5, $05, nRst, $0D, nE5, $05, nRst, $0D
	dc.b nC5, $05, nRst, $07, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nA4, $03, nRst, $01, nA4, $03, nRst, $01
	smpsFMAlterVol	$F8
	dc.b nC5, $05, nRst, $07, nB4, $05, nRst, $0D, nC5, $05, nRst, $0D
	dc.b nD5, $05, nRst, $07, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01, nG5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nG5, $03, nRst, $01
	smpsFMAlterVol	$F4
	dc.b nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$02
	dc.b nE5, $03, nRst, $01, nE5, $03, nRst, $01
	smpsFMAlterVol	$F8
	dc.b nA4, $03, nRst, $09, nF5, $05, nRst, $0D, nE5, $05, nRst, $0D
	dc.b nD5, $05, nRst, $07, nB4, $05, nRst, $0D, nA4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $07
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsAlterNote	$FB
	smpsFMAlterVol	$F8
	dc.b nE5, $0B, nRst, $07, nE5, $03, nRst, $0F
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsPan			panRight, $00
	dc.b nF4, $05, nRst, $01, nG4, $05, nRst, $01, nA4, $05, nRst, $0D
	dc.b nB4, $05, nRst, $0D, nC5, $05, nRst, $07
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsPan			panLeft, $00
	dc.b nE5, $0B, nRst, $07, nE5, $03, nRst, $0F
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsPan			panRight, $00
	dc.b nF4, $05, nRst, $01, nG4, $05, nRst, $01, nA4, $05, nRst, $0D
	dc.b nB4, $05, nRst, $0D, nC5, $05, nRst, $07, nF3, $05, nRst, $01
	dc.b nA3, $05, nRst, $01, nD4, $05, nRst, $01, nF4, $05, nRst, $01
	dc.b nD4, $05, nRst, $01, nF4, $05, nRst, $01, nA4, $05, nRst, $01
	dc.b nD5, $05, nRst, $01, nA4, $05, nRst, $01, nD5, $05, nRst, $01
	dc.b nF5, $05, nRst, $01, nA5, $05, nRst, $01, nF5, $05, nRst, $01
	dc.b nA5, $05, nRst, $01, nD6, $05, nRst, $01, nF6, $05, nRst, $01
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	dc.b nC5, $17, nRst, $01, nB4, $17, nRst, $01, nC5, $17, nRst, $01
	dc.b nD5, $17, nRst, $01
	smpsStop

; ------------------------------------------------------------
; FM Channel 5 - "FM5"
; ------------------------------------------------------------
Snd_AIZ1_Header_FM5:
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	smpsFMAlterVol	$16
	dc.b nE4, $05, nRst, $0D, nE4, $05, nRst, $19, nE4, $05, nRst, $0D
	dc.b nE4, $05, nRst, $19, nD4, $05, nRst, $0D, nD4, $05, nRst, $07
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nBb3, $04, nRst, $02, nC4, $04, nRst, $02, nD4, $04, nRst, $08
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	dc.b nD4, $05, nRst, $07, nD4, $05, nRst, $07, nD4, $05, nRst, $01
	dc.b nEb4, $05, nRst, $07, nE4, $05, nRst, $0D, nE4, $05, nRst, $19
	dc.b nE4, $05, nRst, $0D, nE4, $05, nRst, $19, nD4, $05, nRst, $0D
	dc.b nD4, $05, nRst, $19
	smpsSetvoice	$05
	smpsModSet		$0C, $01, $FA, $05
	smpsAlterNote	$02
	smpsPan			panRight, $00
	dc.b nD4, $02, nRst, $04, nF4, $02, nRst, $04, nD5, $02, nRst, $04
	dc.b nC5, $18, nRst, $06
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	dc.b nE4, $05, nRst, $0D, nE4, $05, nRst, $19, nE4, $05, nRst, $0D
	dc.b nE4, $05, nRst, $19, nD4, $05, nRst, $0D, nD4, $05, nRst, $07
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nBb3, $04, nRst, $02, nC4, $04, nRst, $02, nD4, $04, nRst, $08
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	dc.b nD4, $05, nRst, $07, nD4, $05, nRst, $07, nD4, $05, nRst, $01
	dc.b nEb4, $05, nRst, $07, nE4, $05, nRst, $0D, nE4, $05, nRst, $19
	dc.b nE4, $05, nRst, $0D, nE4, $05, nRst, $19
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	dc.b nA4, $05, nRst, $01, nA4, $05, nRst, $01, nA4, $05, nRst, $01
	dc.b nA4, $05, nRst, $28
	smpsSetvoice	$05
	smpsModSet		$0F, $01, $06, $05
	smpsAlterNote	$03
	smpsPan			panCenter, $00
	smpsFMAlterVol	$04
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb2, $01, nAb3, nBb3, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nD3, $01, nC4, nD4, $39, nRst, $3D
	dc.b nF2, $01, nEb3, nF3, $0A, nBb2, $01, nAb3, nBb3, $0A, nF3, $01
	dc.b nEb4, nF4, $0A, nE3, $01, nD4, nE4, $03, nRst, $0D, nC3, $01
	dc.b nBb3, nC4, $03, nRst, $0D, nBb3, $3B, nRst, $3D, nF3, $0C, nBb3
	dc.b nF4, nE4, $05, nRst, $0D, nC4, $05, nRst, $0D, nD4, $3B, nRst, $6A
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	smpsFMAlterVol	$FC
	dc.b nC4, $0B, nRst, $01, nC4, $0B, nRst, $0D, nBb3, $05, nRst, $0D
	dc.b nC4, $05, nRst, $19
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	smpsFMAlterVol	$08
	dc.b nC6, $0C, nA5, $06, nBb5, $0C, nG5, nC6, nA5, $06, nBb5, $0C
	dc.b nG5, $24
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsFMAlterVol	$F8
	dc.b nC4, $0B, nRst, $01, nC4, $0B, nRst, $0D, nBb3, $05, nRst, $0D
	dc.b nC4, $05, nRst, $19
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	smpsFMAlterVol	$08
	dc.b nC6, $0C, nA5, $06, nBb5, $0C, nG5, nC6, nA5, $06, nBb5, $0C
	dc.b nG5, $24
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsFMAlterVol	$F8
	dc.b nC4, $0B, nRst, $01, nC4, $0B, nRst, $0D, nBb3, $05, nRst, $0D
	dc.b nC4, $05, nRst, $19
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	smpsFMAlterVol	$08
	dc.b nC6, $0C, nA5, $06, nBb5, $0C, nG5, nC6, nA5, $06, nBb5, $0C
	dc.b nG5, $24
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsFMAlterVol	$F8
	dc.b nC4, $0B, nRst, $01, nC4, $0B, nRst, $0D, nBb3, $05, nRst, $0D
	dc.b nC4, $05, nRst, $19
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	smpsFMAlterVol	$08
	dc.b nC6, $0C, nA5, $06, nBb5, $0C, nG5, nC6, $05, nRst, $07
	smpsSetvoice	$04
	smpsModSet		$0F, $01, $06, $06
	smpsAlterNote	$FB
	smpsPan			panRight, $00
	smpsFMAlterVol	$F8
	dc.b nF4, $02, nRst, $04, nG4, $02, nRst, $04, nA4, $02, nRst, $04
	dc.b nC5, $02, nRst, $04, nBb4, $02, nRst, $04, nG4, $02, nRst, $04
	dc.b nA4, $02, nRst, $10
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	smpsAlterNote	$05
	smpsPan			panLeft, $00
	dc.b nG5, $0B, nRst, $01, nC6, $0B, nRst, $01, nG6, $0B, nRst, $01
	dc.b nF6, $11, nRst, $01, nE6, $11, nRst, $01, nC6, $0B, nRst, $01
	dc.b nA5, $23, nRst, $01, nC6, $0B, nRst, $01, nB5, $11, nRst, $01
	dc.b nC6, $11, nRst, $01, nD6, $0B, nRst, $01, nE6, $2F, nRst, $01
	dc.b nA5, $23, nRst, $01, nA5, $0B, nRst, $01, nF6, $11, nRst, $01
	dc.b nE6, $11, nRst, $01, nD6, $0B, nRst, $01, nB5, $11, nRst, $01
	dc.b nA5, $11, nRst, $01, nG5, $17, nRst, $01, nG5, $0B, nRst, $01
	dc.b nC6, $0B, nRst, $01, nG6, $0B, nRst, $01, nF6, $11, nRst, $01
	dc.b nE6, $11, nRst, $01, nC6, $0B, nRst, $01, nA5, $23, nRst, $01
	dc.b nC6, $0B, nRst, $01, nB5, $11, nRst, $01, nC6, $11, nRst, $01
	dc.b nD6, $0B, nRst, $01, nG6, $2F, nRst, $01, nE6, $23, nRst, $01
	dc.b nA5, $0B, nRst, $01, nF6, $11, nRst, $01, nE6, $11, nRst, $01
	dc.b nD6, $0B, nRst, $01, nB5, $11, nRst, $01, nA5, $11, nRst, $01
	dc.b nG5, $0B, nRst, $01
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsPan			panRight, $00
	dc.b nC5, $0B, nRst, $07, nC5, $03, nRst, $0F
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsPan			panLeft, $00
	dc.b nD4, $05, nRst, $01, nE4, $05, nRst, $01, nF4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $0D, nA4, $05, nRst, $07
	smpsSetvoice	$03
	smpsModSet		$0F, $01, $03, $05
	smpsPan			panRight, $00
	dc.b nC5, $0B, nRst, $07, nC5, $03, nRst, $0F
	smpsSetvoice	$07
	smpsModSet		$03, $01, $02, $05
	smpsPan			panLeft, $00
	dc.b nD4, $05, nRst, $01, nE4, $05, nRst, $01, nF4, $05, nRst, $0D
	dc.b nG4, $05, nRst, $0D, nA4, $05, nRst, $07
	smpsSetvoice	$08
	smpsModSet		$0F, $01, $FA, $06
	dc.b nD6, $24, nD6, $06, nE6, nF6, $12, nE6, nD6, $0C, nG6, $60
	smpsStop

; ------------------------------------------------------------
; PSG Channel 1 - "PSG1"
; ------------------------------------------------------------
Snd_AIZ1_Header_PSG1:
	smpsPSGvoice		sTone_0C
	dc.b nG2, $05, nRst, $0D, nG2, $05, nRst, $19, nG2, $05, nRst, $0D
	dc.b nG2, $05, nRst, $19, nF2, $05, nRst, $0D, nF2, $05, nRst, $07
	dc.b nD2, $02, nRst, $04, nE2, $02, nRst, $04, nF2, $02, nRst, $0A
	dc.b nF2, $05, nRst, $07, nF2, $05, nRst, $07, nF2, $05, nRst, $01
	dc.b nFs2, $05, nRst, $07, nG2, $05, nRst, $0D, nG2, $05, nRst, $19
	dc.b nG2, $05, nRst, $0D, nG2, $05, nRst, $19, nC3, $03, nRst, nC3
	dc.b nRst, nC4, nRst, nC3, nRst, nBb3, nRst, nC3, nRst, nBb3, nRst, nC4
	dc.b nRst, nF3, $02, nRst, $04, nBb3, $02, nRst, $04, nF4, $02, nRst, $04
	dc.b nE4, $1D, nRst, $01, nG2, $05, nRst, $0D, nG2, $05, nRst, $19
	dc.b nG2, $05, nRst, $0D, nG2, $05, nRst, $19, nF2, $05, nRst, $0D
	dc.b nF2, $05, nRst, $07, nD2, $02, nRst, $04, nE2, $02, nRst, $04
	dc.b nF2, $02, nRst, $0A, nF2, $05, nRst, $07, nF2, $05, nRst, $07
	dc.b nF2, $05, nRst, $01, nFs2, $05, nRst, $07, nG2, $05, nRst, $0D
	dc.b nG2, $05, nRst, $19, nG2, $05, nRst, $0D, nG2, $05, nRst, $19
	dc.b nC5, $05, nRst, $01, nC5, $05, nRst, $01, nC5, $05, nRst, $01
	dc.b nC5, $05, nRst, $55, nD2, $02, nRst, $0A, nE2, $02, nRst, $16
	dc.b nD2, $11, nRst, $01, nE2, $02, nRst, $28, nA3, $05, nRst, $01
	dc.b nBb3, $05, nRst, $07, nBb3, $02, nRst, $04, nBb3, $02, nRst, $04
	dc.b nBb3, $02, nRst, $04, nBb3, $02, nRst, $0A, nA3, $23, nRst, $0D
	dc.b nD2, $02, nRst, $0A, nE2, $02, nRst, $16, nD2, $11, nRst, $01
	dc.b nE2, $02, nRst, $28, nA3, $05, nRst, $01, nBb3, $05, nRst, $07
	dc.b nBb3, $02, nRst, $04, nBb3, $02, nRst, $04, nBb3, $02, nRst, $04
	dc.b nBb3, $02, nRst, $0A, nC4, $05, nRst, $01, nBb3, $1D, nRst, $0D
	dc.b nD2, $02, nRst, $0A, nE2, $02, nRst, $16, nD2, $11, nRst, $01
	dc.b nE2, $02, nRst, $28, nA3, $05, nRst, $01, nBb3, $05, nRst, $07
	dc.b nBb3, $02, nRst, $04, nBb3, $02, nRst, $04, nBb3, $02, nRst, $04
	dc.b nBb3, $02, nRst, $0A, nA3, $23, nRst, $0D, nD2, $02, nRst, $0A
	dc.b nE2, $02, nRst, $16, nD2, $11, nRst, $01, nE2, $02, nRst, $28
	dc.b nA3, $05, nRst, $01, nBb3, $05, nRst, $07, nBb3, $02, nRst, $04
	dc.b nBb3, $02, nRst, $04, nBb3, $02, nRst, $04, nBb3, $02, nRst, $0A
	dc.b nC4, $05, nRst, $01, nBb3, $1D, nRst, $0D, nC2, $0B, nRst, $01
	dc.b nC2, $0B, nRst, $0D, nBb1, $05, nRst, $0D, nC2, $05, nRst, $19
	dc.b nC4, $0C, nA3, $06, nBb3, $0C, nG3, nC4, nA3, $06, nBb3, $0C
	dc.b nG3, $24, nC2, $0B, nRst, $01, nC2, $0B, nRst, $0D, nBb1, $05
	dc.b nRst, $0D, nC2, $05, nRst, $19, nC4, $0C, nA3, $06, nBb3, $0C
	dc.b nG3, nC4, nA3, $06, nBb3, $0C, nG3, $24, nC2, $0B, nRst, $01
	dc.b nC2, $0B, nRst, $0D, nBb1, $05, nRst, $0D, nC2, $05, nRst, $19
	dc.b nC4, $0C, nA3, $06, nBb3, $0C, nG3, nC4, nA3, $06, nBb3, $0C
	dc.b nG3, $24, nC2, $0B, nRst, $01, nC2, $0B, nRst, $0D, nBb1, $05
	dc.b nRst, $0D, nC2, $05, nRst, $19, nC4, $0C, nA3, $06, nBb3, $0C
	dc.b nG3, nC4, nF2, $02, nRst, $04, nG2, $02, nRst, $04, nA2, $02
	dc.b nRst, $04, nC3, $02, nRst, $04, nBb2, $02, nRst, $04, nG2, $02
	dc.b nRst, $04, nA2, $02, nRst, $04, nD4, $03, nRst, nB3, nRst, nG3
	dc.b nRst, nE3, nRst, nD3, nRst, nB2, nRst, nG2, nRst, nE2, nRst, nC4
	dc.b nRst, nB3, nRst, nG3, nRst, nE3, nRst, nC3, nRst, nB2, nRst, nG2
	dc.b nRst, nE2, nRst, nC4, nRst, nA3, nRst, nF3, nRst, nD3, nRst, nC3
	dc.b nRst, nA2, nRst, nF2, nRst, nD2, nRst, nB3, nRst, nA3, nRst, nF3
	dc.b nRst, nD3, nRst, nB2, nRst, nA2, nRst, nF2, nRst, nD2, nRst, $0F
	dc.b nG2, $0B, nRst, $01, nD3, $0B, nRst, $01, nG3, $0B, nRst, $01
	dc.b nF3, $05, nRst, $0D, nE3, $05, nRst, $0D, nC3, $05, nRst, $07
	dc.b nA2, $2F, nRst, $01, nD3, $05, nRst, $0D, nC3, $05, nRst, $0D
	dc.b nB2, $05, nRst, $07, nD4, $03, nRst, nB3, nRst, nG3, nRst, nE3
	dc.b nRst, nD3, nRst, nB2, nRst, nG2, nRst, nE2, nRst, nC4, nRst, nB3
	dc.b nRst, nG3, nRst, nE3, nRst, nC3, nRst, nB2, nRst, nG2, nRst, nE2
	dc.b nRst, nC4, nRst, nA3, nRst, nF3, nRst, nD3, nRst, nC3, nRst, nA2
	dc.b nRst, nF2, nRst, nD2, nRst, nB3, nRst, nA3, nRst, nF3, nRst, nD3
	dc.b nRst, nB2, nRst, nA2, nRst, nF2, nRst, nD2, nRst, nG3, $2F, nRst, $01
	dc.b nE3, $23, nRst, $01, nA2, $0B, nRst, $01, nF3, $11, nRst, $01
	dc.b nE3, $11, nRst, $01, nD3, $0B, nRst, $01, nB2, $11, nRst, $01
	dc.b nA2, $11, nRst, $01, nG2, $0B, nRst, $01, nE3, $0B, nRst, $07
	dc.b nE3, $03, nRst, $0F, nA2, $02, nRst, $04, nB2, $02, nRst, $04
	dc.b nC3, $11, nRst, $01, nB2, $11, nRst, $01, nA2, $0B, nRst, $01
	dc.b nE3, $0B, nRst, $07, nE3, $03, nRst, $0F, nF2, $02, nRst, $04
	dc.b nG2, $02, nRst, $04, nA2, $11, nRst, $01, nG2, $11, nRst, $01
	dc.b nF2, $0B, nRst, $01, nF1, $05, nRst, $01, nA1, $05, nRst, $01
	dc.b nD2, $05, nRst, $01, nF2, $05, nRst, $01, nD2, $05, nRst, $01
	dc.b nF2, $05, nRst, $01, nA2, $05, nRst, $01, nD3, $05, nRst, $01
	dc.b nA2, $05, nRst, $01, nD3, $05, nRst, $01, nF3, $05, nRst, $01
	dc.b nA3, $05, nRst, $01, nF3, $05, nRst, $01, nA3, $05, nRst, $01
	dc.b nD4, $05, nRst, $01, nF4, $05, nRst, $01, nC3, $17, nRst, $01
	dc.b nB2, $17, nRst, $01, nC3, $17, nRst, $01, nD3, $17, nRst, $01
	smpsStop

; ------------------------------------------------------------
; PSG Channel 2 - "PSG2"
; ------------------------------------------------------------
Snd_AIZ1_Header_PSG2:
	smpsPSGvoice		sTone_0C
	dc.b nE2, $05, nRst, $0D, nE2, $05, nRst, $19, nE2, $05, nRst, $0D
	dc.b nE2, $05, nRst, $19, nD2, $05, nRst, $0D, nD2, $05, nRst, $07
	dc.b nBb1, $02, nRst, $04, nC2, $02, nRst, $04, nD2, $02, nRst, $0A
	dc.b nD2, $05, nRst, $07, nD2, $05, nRst, $07, nD2, $05, nRst, $01
	dc.b nEb2, $05, nRst, $07, nE2, $05, nRst, $0D, nE2, $05, nRst, $19
	dc.b nE2, $05, nRst, $0D, nE2, $05, nRst, $19, nC3, $03, nRst, nC3
	dc.b nRst, nC4, nRst, nC3, nRst, nBb3, nRst, nC3, nRst, nBb3, nRst, nC4
	dc.b nRst, nF3, $02, nRst, $04, nBb3, $02, nRst, $04, nF4, $02, nRst, $04
	dc.b nE4, $1D, nRst, $01, nE2, $05, nRst, $0D, nE2, $05, nRst, $19
	dc.b nE2, $05, nRst, $0D, nE2, $05, nRst, $19, nD2, $05, nRst, $0D
	dc.b nD2, $05, nRst, $07, nBb1, $02, nRst, $04, nC2, $02, nRst, $04
	dc.b nD2, $02, nRst, $0A, nD2, $05, nRst, $07, nD2, $05, nRst, $07
	dc.b nD2, $05, nRst, $01, nEb2, $05, nRst, $07, nE2, $05, nRst, $0D
	dc.b nE2, $05, nRst, $19, nE2, $05, nRst, $0D, nE2, $05, nRst, $19
	dc.b nA3, $05, nRst, $01, nA3, $05, nRst, $01, nA3, $05, nRst, $01
	dc.b nA3, $05, nRst, $55, nBb1, $02, nRst, $0A, nC2, $02, nRst, $16
	dc.b nBb1, $11, nRst, $01, nC2, $02, nRst, $28, nFs3, $05, nRst, $01
	dc.b nG3, $05, nRst, $07, nG3, $02, nRst, $04, nG3, $02, nRst, $04
	dc.b nG3, $02, nRst, $04, nG3, $02, nRst, $0A, nF3, $23, nRst, $0D
	dc.b nBb1, $02, nRst, $0A, nC2, $02, nRst, $16, nBb1, $11, nRst, $01
	dc.b nC2, $02, nRst, $28, nFs3, $05, nRst, $01, nG3, $05, nRst, $07
	dc.b nG3, $02, nRst, $04, nG3, $02, nRst, $04, nG3, $02, nRst, $04
	dc.b nG3, $02, nRst, $0A, nA3, $05, nRst, $01, nG3, $1D, nRst, $0D
	dc.b nBb1, $02, nRst, $0A, nC2, $02, nRst, $16, nBb1, $11, nRst, $01
	dc.b nC2, $02, nRst, $28, nFs3, $05, nRst, $01, nG3, $05, nRst, $07
	dc.b nG3, $02, nRst, $04, nG3, $02, nRst, $04, nG3, $02, nRst, $04
	dc.b nG3, $02, nRst, $0A, nF3, $23, nRst, $0D, nBb1, $02, nRst, $0A
	dc.b nC2, $02, nRst, $16, nBb1, $11, nRst, $01, nC2, $02, nRst, $28
	dc.b nFs3, $05, nRst, $01, nG3, $05, nRst, $07, nG3, $02, nRst, $04
	dc.b nG3, $02, nRst, $04, nG3, $02, nRst, $04, nG3, $02, nRst, $0A
	dc.b nA3, $05, nRst, $01, nG3, $1D, nRst, $0D, nC2, $0B, nRst, $01
	dc.b nC2, $0B, nRst, $0D, nBb1, $05, nRst, $0D, nC2, $05, nRst, $19
	dc.b nC4, $0C, nA3, $06, nBb3, $0C, nG3, nC4, nA3, $06, nBb3, $0C
	dc.b nG3, $24, nC2, $0B, nRst, $01, nC2, $0B, nRst, $0D, nBb1, $05
	dc.b nRst, $0D, nC2, $05, nRst, $19, nC4, $0C, nA3, $06, nBb3, $0C
	dc.b nG3, nC4, nA3, $06, nBb3, $0C, nG3, $24, nC2, $0B, nRst, $01
	dc.b nC2, $0B, nRst, $0D, nBb1, $05, nRst, $0D, nC2, $05, nRst, $19
	dc.b nC4, $0C, nA3, $06, nBb3, $0C, nG3, nC4, nA3, $06, nBb3, $0C
	dc.b nG3, $24, nC2, $0B, nRst, $01, nC2, $0B, nRst, $0D, nBb1, $05
	dc.b nRst, $0D, nC2, $05, nRst, $19, nC4, $0C, nA3, $06, nBb3, $0C
	dc.b nG3, nC4, nF2, $02, nRst, $04, nG2, $02, nRst, $04, nA2, $02
	dc.b nRst, $04, nC3, $02, nRst, $04, nBb2, $02, nRst, $04, nG2, $02
	dc.b nRst, $04, nA2, $02, nRst, $04, nD4, $03, nRst, nB3, nRst, nG3
	dc.b nRst, nE3, nRst, nD3, nRst, nB2, nRst, nG2, nRst, nE2, nRst, nC4
	dc.b nRst, nB3, nRst, nG3, nRst, nE3, nRst, nC3, nRst, nB2, nRst, nG2
	dc.b nRst, nE2, nRst, nC4, nRst, nA3, nRst, nF3, nRst, nD3, nRst, nC3
	dc.b nRst, nA2, nRst, nF2, nRst, nD2, nRst, nB3, nRst, nA3, nRst, nF3
	dc.b nRst, nD3, nRst, nB2, nRst, nA2, nRst, nF2, nRst, nD2, nRst, $15
	dc.b nD2, $0B, nRst, $01, nD3, $0B, nRst, $01, nG3, $0B, nRst, $01
	dc.b nF3, $05, nRst, $0D, nE3, $05, nRst, $0D, nC3, $05, nRst, $07
	dc.b nA2, $2F, nRst, $01, nD3, $05, nRst, $0D, nC3, $05, nRst, $0D
	dc.b nB2, $05, nRst, $01, nB2, $03, nRst, nB3, nRst, nG3, nRst, nE3
	dc.b nRst, nD3, nRst, nB2, nRst, nG2, nRst, nE2, nRst, nC4, nRst, nB3
	dc.b nRst, nG3, nRst, nE3, nRst, nC3, nRst, nB2, nRst, nG2, nRst, nE2
	dc.b nRst, nC4, nRst, nA3, nRst, nF3, nRst, nD3, nRst, nC3, nRst, nA2
	dc.b nRst, nF2, nRst, nD2, nRst, nB3, nRst, nA3, nRst, nF3, nRst, nD3
	dc.b nRst, nB2, nRst, nA2, nRst, nF2, nRst, nD2, nRst, $09, nG3, $2F
	dc.b nRst, $01, nE3, $23, nRst, $01, nA2, $0B, nRst, $01, nF3, $11
	dc.b nRst, $01, nE3, $11, nRst, $01, nD3, $0B, nRst, $01, nB2, $11
	dc.b nRst, $01, nA2, $11, nRst, $01, nG2, $06, nC3, $0B, nRst, $07
	dc.b nC3, $03, nRst, $0F, nA2, $02, nRst, $04, nB2, $02, nRst, $04
	dc.b nC3, $11, nRst, $01, nB2, $11, nRst, $01, nA2, $0B, nRst, $01
	dc.b nC3, $0B, nRst, $07, nC3, $03, nRst, $0F, nF2, $02, nRst, $04
	dc.b nG2, $02, nRst, $04, nA2, $11, nRst, $01, nG2, $11, nRst, $01
	dc.b nF2, $0B, nRst, $01, nF1, $05, nRst, $01, nA1, $05, nRst, $01
	dc.b nD2, $05, nRst, $01, nF2, $05, nRst, $01, nD2, $05, nRst, $01
	dc.b nF2, $05, nRst, $01, nA2, $05, nRst, $01, nD3, $05, nRst, $01
	dc.b nA2, $05, nRst, $01, nD3, $05, nRst, $01, nF3, $05, nRst, $01
	dc.b nA3, $05, nRst, $01, nF3, $05, nRst, $01, nA3, $05, nRst, $01
	dc.b nD4, $05, nRst, $01, nF4, $05, nRst, $01, nC3, $17, nRst, $01
	dc.b nB2, $17, nRst, $01, nC3, $17, nRst, $01, nD3, $17, nRst, $01
	smpsStop

; ------------------------------------------------------------
; PSG_Noise Channel 4 - "PSG3 (Noise)"
; ------------------------------------------------------------
Snd_AIZ1_Header_PSG_Noise4:
	smpsPSGform		$E7
	smpsPSGvoice		sTone_02
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6, nA6, $6C, nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6, nA6, $6C, nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C
	smpsPSGvoice		sTone_01
	dc.b nBb6, $06, nBb6
	smpsPSGvoice		sTone_04
	dc.b nBb6, $0C, nRst, $60
	smpsStop

