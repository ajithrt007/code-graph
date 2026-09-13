import { useUpdater } from "../updater/useUpdater";

/**
 * Self-contained update UI: checks GitHub Releases on mount (silently —
 * availability only, never installs) and offers a manual re-check.
 * Rendered inside the projects screen; unobtrusive by design.
 */
export function UpdateSection() {
  const {
    appVersion,
    status,
    update,
    progress,
    error,
    checkForUpdates,
    downloadAndInstall,
    restartNow,
  } = useUpdater(true);

  return (
    <div className="update-section" aria-label="Application updates">
      <div className="update-section__row">
        <span className="update-section__version">Version {appVersion}</span>
        {(status === "idle" || status === "up-to-date" || status === "error") && (
          <button
            type="button"
            className="update-section__button"
            onClick={() => void checkForUpdates()}
          >
            Check for updates
          </button>
        )}
      </div>

      {status === "checking" && (
        <p role="status" className="update-section__note">
          Checking for updates…
        </p>
      )}

      {status === "up-to-date" && (
        <p role="status" className="update-section__note">
          You&apos;re up to date.
        </p>
      )}

      {status === "available" && update && (
        <div className="update-section__available">
          <p role="status">
            Version {update.version} is available.
          </p>
          {update.body && (
            <details className="update-section__notes">
              <summary>Release notes</summary>
              <pre>{update.body}</pre>
            </details>
          )}
          <button
            type="button"
            className="update-section__button update-section__button--primary"
            onClick={() => void downloadAndInstall()}
          >
            Update now
          </button>
        </div>
      )}

      {status === "downloading" && (
        <div className="update-section__download">
          <p role="status">Downloading update…</p>
          <div
            className="load-terminal__bar"
            aria-label={`Download progress ${progress ?? 0}%`}
          >
            <div
              className="load-terminal__fill"
              style={{ width: `${progress ?? 0}%` }}
            />
          </div>
          <p className="update-section__note">{progress ?? 0}%</p>
        </div>
      )}

      {status === "installed" && (
        <div className="update-section__available">
          <p role="status">Update installed.</p>
          <button
            type="button"
            className="update-section__button update-section__button--primary"
            onClick={() => void restartNow()}
          >
            Restart now
          </button>
        </div>
      )}

      {status === "error" && error && (
        <p role="alert" className="update-section__error">
          Update check failed: {error}
        </p>
      )}
    </div>
  );
}
