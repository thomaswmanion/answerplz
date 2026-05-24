import "./DisplayPreview.css";

function displayNumberFromLabel(label: string): number {
  const index = Number.parseInt(label.replace("monitor-preview-", ""), 10);
  return Number.isNaN(index) ? 1 : index + 1;
}

export function DisplayPreview({ windowLabel }: { windowLabel: string }) {
  const displayNumber = displayNumberFromLabel(windowLabel);

  return (
    <div className="display-preview" aria-hidden>
      <div className="display-preview__frame">
        <span className="display-preview__number">{displayNumber}</span>
      </div>
    </div>
  );
}
