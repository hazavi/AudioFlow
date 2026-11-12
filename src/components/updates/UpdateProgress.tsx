import { bytes } from "../../lib/utils";
import { useUpdateStore } from "../../stores/useUpdateStore";
export function UpdateProgress() {
  const { downloaded, total } = useUpdateStore();
  const percent = total ? Math.min(100, (downloaded / total) * 100) : undefined;
  return (
    <div className="update-progress">
      <div
        role="progressbar"
        aria-label="Update download"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className={`progress-track ${percent === undefined ? "indeterminate" : ""}`}
      >
        <div
          style={percent === undefined ? undefined : { width: `${percent}%` }}
        />
      </div>
      <span className="label">
        Downloaded: {bytes(downloaded)}
        {total ? ` / ${bytes(total)}` : " · total size unavailable"}
      </span>
    </div>
  );
}
