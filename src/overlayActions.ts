import { invoke } from "@tauri-apps/api/core";

export async function requestScreenshotAnswer(): Promise<string> {
  const res = await invoke<{ answer: string }>("capture_and_answer");
  return res.answer;
}

export async function requestQuestionAnswer(question: string): Promise<string> {
  const res = await invoke<{ answer: string }>("answer_question", { question });
  return res.answer;
}

export async function requestClipboardAnswer(): Promise<string> {
  const res = await invoke<{ answer: string }>("answer_from_clipboard");
  return res.answer;
}
