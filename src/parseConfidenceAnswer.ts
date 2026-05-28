export interface ConfidenceAnswer {
  confidence: number | null;
  answer: string;
  raw: string;
}

/** Parse model output in the form `85%, answer text`. Falls back to raw text if unparsable. */
export function parseConfidenceAnswer(raw: string): ConfidenceAnswer {
  const trimmed = raw.trim();
  const match = /^(\d{1,3})%\s*,\s*(.+)$/s.exec(trimmed);
  if (!match) {
    return { confidence: null, answer: trimmed, raw: trimmed };
  }

  const confidence = Math.min(100, Math.max(0, Number.parseInt(match[1], 10)));
  return { confidence, answer: match[2].trim(), raw: trimmed };
}
