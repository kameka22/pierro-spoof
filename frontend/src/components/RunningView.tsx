import { SpooferState } from '../types';

interface RunningViewProps {
  state: SpooferState | null;
  onStop: () => void;
}

function RunningView({ state, onStop }: RunningViewProps) {

  const formatBytes = (bytes: number): string => {
    if (bytes >= 1024 ** 4) return `${(bytes / 1024 ** 4).toFixed(2)} TB`;
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(2)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    return `${bytes} B`;
  };

  const formatSpeed = (bytesPerSec: number): string => {
    if (bytesPerSec >= 1024 ** 2) return `${(bytesPerSec / 1024 ** 2).toFixed(1)} MB/s`;
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(0)} KB/s`;
    return `${bytesPerSec} B/s`;
  };

  const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const getStatusBadgeClass = (status: any): string => {
    if (status === 'Running') return 'running';
    if (status === 'Starting') return 'starting';
    return 'stopped';
  };

  const getStatusText = (status: any): string => {
    if (typeof status === 'string') return status;
    if (status && typeof status === 'object' && 'Error' in status) {
      return `Error: ${status.Error}`;
    }
    return 'Unknown';
  };

  if (!state) {
    return (
      <div className="loading-state">
        <div className="loading-spinner"></div>
        <p>Initialisation...</p>
      </div>
    );
  }

  const downloadPercent = state.total_size > 0
    ? ((state.current_downloaded / state.total_size) * 100).toFixed(1)
    : '0';

  const uploadPercent = state.total_size > 0
    ? ((state.current_uploaded / state.total_size) * 100).toFixed(1)
    : '0';

  return (
    <div className="view-enter">
      <div className="status-display">
        <div className="status-header">
          <h3>{state.torrent_name}</h3>
          <span className={`status-badge ${getStatusBadgeClass(state.status)}`}>
            {getStatusText(state.status)}
          </span>
        </div>

        {/* Config summary */}
        <div className="config-summary">
          <div className="config-row">
            <span className="config-label">Client</span>
            <span className="config-value">{state.client_profile}</span>
          </div>
          <div className="config-row">
            <span className="config-label">Peer ID</span>
            <span className="config-value mono">{state.peer_id}</span>
          </div>
          <div className="config-row">
            <span className="config-label">Vitesses</span>
            <span className="config-value">
              ↓ {formatSpeed(state.download_speed)} / ↑ {formatSpeed(state.upload_speed)}
              {state.realistic_mode && <span className="variance-tag">±{state.variance_percent}%</span>}
            </span>
          </div>
          {state.target_ratio && (
            <div className="config-row">
              <span className="config-label">Ratio cible</span>
              <span className="config-value highlight">{state.target_ratio.toFixed(2)}</span>
            </div>
          )}
          {state.peer_rotation_minutes && (
            <div className="config-row">
              <span className="config-label">Rotation peer</span>
              <span className="config-value">
                Toutes les {state.peer_rotation_minutes} min
                {state.peer_rotation_count > 0 && <span className="rotation-count">({state.peer_rotation_count} effectuées)</span>}
              </span>
            </div>
          )}
        </div>

        <div className="tracker-info">
          <span className="tracker-label">Tracker</span>
          <span className="tracker-value">{state.current_tracker}</span>
        </div>

        {/* Feature badges */}
        <div className="feature-badges">
          {state.realistic_mode && (
            <span className="feature-badge active">Mode Réaliste</span>
          )}
          {state.peer_rotation_count > 0 && (
            <span className="feature-badge active">Rotations: {state.peer_rotation_count}</span>
          )}
        </div>

        <div className="stats-grid">
          <div className="stat-item">
            <div className="stat-label">Ratio</div>
            <div className="stat-value ratio-display">
              <span className="ratio-value">{state.current_ratio.toFixed(2)}</span>
              {state.target_ratio && (
                <span className="ratio-target">/ {state.target_ratio.toFixed(1)}</span>
              )}
            </div>
          </div>
          <div className="stat-item">
            <div className="stat-label">Seeders / Leechers</div>
            <div className="stat-value">{state.seeders} / {state.leechers}</div>
          </div>
        </div>

        <div className="progress-container">
          <div className="progress-label">
            <span>Downloaded</span>
            <span>{formatBytes(state.current_downloaded)} / {formatBytes(state.total_size)}</span>
          </div>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${downloadPercent}%` }}></div>
          </div>
        </div>

        <div className="progress-container">
          <div className="progress-label">
            <span>Uploaded</span>
            <span>{formatBytes(state.current_uploaded)} ({uploadPercent}%)</span>
          </div>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${Math.min(parseFloat(uploadPercent), 100)}%` }}></div>
          </div>
        </div>

        <div className="next-announce">
          <span className="announce-label">Prochain announce</span>
          <span className="announce-time">{formatTime(state.next_announce_in)}</span>
        </div>

        {state.history && state.history.length > 0 && (
          <div className="history">
            <h3>Historique</h3>
            <div className="history-list">
              {state.history.slice().reverse().map((update, idx) => (
                <div key={idx} className="history-item">
                  <span className="time">{new Date(update.timestamp).toLocaleTimeString()}</span>
                  <span className="data">
                    <span className="download">↓ {formatBytes(update.downloaded)}</span>
                    <span className="upload">↑ {formatBytes(update.uploaded)}</span>
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="button-group">
        <button className="danger" onClick={onStop}>
          Arrêter le spoofing
        </button>
      </div>
    </div>
  );
}

export default RunningView;
