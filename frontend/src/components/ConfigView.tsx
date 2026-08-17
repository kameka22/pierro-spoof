import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { SpooferConfig, ParsedTorrentInfo } from '../types';

interface ConfigViewProps {
  onStart: (config: SpooferConfig) => void;
}

function ConfigView({ onStart }: ConfigViewProps) {
  const [torrentPath, setTorrentPath] = useState('');
  const [torrentInfo, setTorrentInfo] = useState<ParsedTorrentInfo | null>(null);
  const [clientProfile, setClientProfile] = useState('qbit-4.3.3');
  const [availableClients, setAvailableClients] = useState<[string, string][]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const [initialDownloaded, setInitialDownloaded] = useState('0%');
  const [initialUploaded, setInitialUploaded] = useState('0%');
  const [downloadSpeed, setDownloadSpeed] = useState('100');
  const [uploadSpeed, setUploadSpeed] = useState('1000');
  const [port, setPort] = useState('8999');

  const [error, setError] = useState('');

  useEffect(() => {
    loadAvailableClients();
  }, []);

  const loadAvailableClients = async () => {
    try {
      const clients = await invoke<[string, string][]>('get_available_clients');
      setAvailableClients(clients);
    } catch (err) {
      console.error('Failed to load clients:', err);
    }
  };

  const handleBrowse = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: 'Torrent',
            extensions: ['torrent']
          }
        ]
      });

      if (selected) {
        setTorrentPath(selected);
        // Parse torrent to get info
        try {
          const info = await invoke<ParsedTorrentInfo>('parse_torrent', { path: selected });
          setTorrentInfo(info);
          setError('');
        } catch (err) {
          setError(`Failed to parse torrent: ${err}`);
          setTorrentInfo(null);
        }
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
    }
  };

  const handleStart = async () => {
    if (!torrentPath || !torrentInfo) {
      setError('Please select a torrent file');
      return;
    }

    try {
      // Parse size inputs
      const downloaded = await invoke<number>('parse_size_input', {
        input: initialDownloaded,
        totalSize: torrentInfo.total_size
      });

      const uploaded = await invoke<number>('parse_size_input', {
        input: initialUploaded,
        totalSize: torrentInfo.total_size
      });

      // Parse speed inputs
      const downloadSpeedBytes = await invoke<number>('parse_speed_input', {
        input: downloadSpeed
      });

      const uploadSpeedBytes = await invoke<number>('parse_speed_input', {
        input: uploadSpeed
      });

      const config: SpooferConfig = {
        torrent_path: torrentPath,
        client_profile: clientProfile,
        initial_downloaded: downloaded,
        initial_uploaded: uploaded,
        download_speed: downloadSpeedBytes,
        upload_speed: uploadSpeedBytes,
        port: parseInt(port)
      };

      onStart(config);
    } catch (err) {
      setError(`Configuration error: ${err}`);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes >= 1024 ** 4) return `${(bytes / 1024 ** 4).toFixed(2)} TB`;
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(2)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    return `${bytes} B`;
  };

  return (
    <div>
      {error && <div className="error-message">{error}</div>}

      <div className="form-group">
        <label>Fichier Torrent</label>
        <div className="file-input-group">
          <input
            type="text"
            value={torrentPath}
            readOnly
            placeholder="Sélectionner un fichier .torrent"
          />
          <button onClick={handleBrowse}>Parcourir</button>
        </div>
      </div>

      {torrentInfo && (
        <div className="torrent-info">
          <p><strong>Nom</strong> <span>{torrentInfo.name}</span></p>
          <p><strong>Taille</strong> <span>{formatBytes(torrentInfo.total_size)}</span></p>
          <p><strong>Trackers</strong> <span>{torrentInfo.num_trackers}</span></p>
        </div>
      )}

      <div
        className={`advanced-toggle ${showAdvanced ? 'open' : ''}`}
        onClick={() => setShowAdvanced(!showAdvanced)}
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
        {showAdvanced ? 'Masquer' : 'Afficher'} les paramètres additionnels
      </div>

      {showAdvanced && (
        <div className="advanced-options">
          <div className="form-group">
            <label>Client à émuler</label>
            <select value={clientProfile} onChange={(e) => setClientProfile(e.target.value)}>
              {availableClients.map(([id, name]) => (
                <option key={id} value={id}>{name}</option>
              ))}
            </select>
          </div>

          <div className="input-row">
            <div className="form-group">
              <label>Téléchargement Initial</label>
              <input
                type="text"
                value={initialDownloaded}
                onChange={(e) => setInitialDownloaded(e.target.value)}
                placeholder="ex: 90%, 5GB, 500MB"
              />
            </div>

            <div className="form-group">
              <label>Vitesse Download (KB/s)</label>
              <input
                type="text"
                value={downloadSpeed}
                onChange={(e) => setDownloadSpeed(e.target.value)}
                placeholder="ex: 100"
              />
            </div>
          </div>

          <div className="input-row">
            <div className="form-group">
              <label>Upload Initial</label>
              <input
                type="text"
                value={initialUploaded}
                onChange={(e) => setInitialUploaded(e.target.value)}
                placeholder="ex: 0%, 1GB, 500MB"
              />
            </div>

            <div className="form-group">
              <label>Vitesse Upload (KB/s)</label>
              <input
                type="text"
                value={uploadSpeed}
                onChange={(e) => setUploadSpeed(e.target.value)}
                placeholder="ex: 1000"
              />
            </div>
          </div>

          <div className="form-group">
            <label>Port</label>
            <input
              type="number"
              value={port}
              onChange={(e) => setPort(e.target.value)}
              placeholder="8999"
            />
          </div>
        </div>
      )}

      <div className="button-group">
        <button onClick={handleStart} disabled={!torrentInfo}>
          Démarrer
        </button>
      </div>
    </div>
  );
}

export default ConfigView;
