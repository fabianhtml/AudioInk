// AudioInk - Main JavaScript
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open, ask } = window.__TAURI__.dialog;

// State - separated by tab
let activeTab = 'file';
let isProcessing = false;
let isDownloading = false;
let settingsOpen = false;

// File tab state
let fileState = {
    path: null,
    name: null
};

// YouTube tab state
let youtubeState = {
    url: null,
    videoId: null,
    title: null,
    method: null, // 'captions' or 'whisper'
    hasCaptions: false,
    captionLanguages: []
};

// yt-dlp availability
let ytdlpAvailable = false;

// History expanded state
let expandedHistoryId = null;

// Settings persistence
const STORAGE_KEY = 'audioink_settings';
const defaultSettings = {
    preferredModel: 'base',
    preferredLanguage: 'auto',
    includeTimestamps: false
};

function loadSettings() {
    try {
        const saved = localStorage.getItem(STORAGE_KEY);
        return saved ? { ...defaultSettings, ...JSON.parse(saved) } : defaultSettings;
    } catch {
        return defaultSettings;
    }
}

function saveSettings(settings) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

// DOM Elements
const elements = {};

// Initialize app
window.addEventListener("DOMContentLoaded", async () => {
    initElements();
    initEventListeners();
    applySettings();
    await checkModelStatus();
    await checkYtdlpAvailable();
    await loadHistory();
});

// Check if yt-dlp is available
async function checkYtdlpAvailable() {
    try {
        ytdlpAvailable = await invoke('check_ytdlp_available');
    } catch (error) {
        console.error('Error checking yt-dlp:', error);
        ytdlpAvailable = false;
    }
}

function applySettings() {
    const settings = loadSettings();
    elements.modelSelect.value = settings.preferredModel;
    elements.languageSelect.value = settings.preferredLanguage;
    elements.includeTimestamps.checked = settings.includeTimestamps;
    updateModelIndicator();
}

function initElements() {
    // Tabs
    elements.tabs = document.querySelectorAll('.tab');
    elements.tabContents = document.querySelectorAll('.tab-content');

    // File tab
    elements.fileDropZone = document.getElementById('file-drop-zone');
    elements.fileSelected = document.getElementById('file-selected');
    elements.fileName = document.getElementById('file-name');
    elements.clearFile = document.getElementById('clear-file');

    // YouTube tab
    elements.youtubeInputSection = document.getElementById('youtube-input-section');
    elements.youtubeUrl = document.getElementById('youtube-url');
    elements.loadYoutubeBtn = document.getElementById('load-youtube-btn');
    elements.youtubeOptions = document.getElementById('youtube-options');
    elements.youtubeVideoTitle = document.getElementById('youtube-video-title');
    elements.useYoutubeCaptions = document.getElementById('use-youtube-captions');
    elements.useWhisper = document.getElementById('use-whisper');
    elements.captionsStatus = document.getElementById('captions-status');
    elements.youtubeSelected = document.getElementById('youtube-selected');
    elements.youtubeSelectedName = document.getElementById('youtube-selected-name');
    elements.youtubeSelectedMethod = document.getElementById('youtube-selected-method');
    elements.clearYoutube = document.getElementById('clear-youtube');

    // Settings
    elements.settingsBtn = document.getElementById('settings-btn');
    elements.settingsOverlay = document.getElementById('settings-overlay');
    elements.closeSettings = document.getElementById('close-settings');
    elements.modelSelect = document.getElementById('model-select');
    elements.modelStatus = document.getElementById('model-status');
    elements.downloadModelBtn = document.getElementById('download-model-btn');
    elements.downloadProgress = document.getElementById('download-progress');
    elements.downloadProgressFill = document.getElementById('download-progress-fill');
    elements.downloadProgressText = document.getElementById('download-progress-text');
    elements.downloadedModelsList = document.getElementById('downloaded-models-list');
    elements.includeTimestamps = document.getElementById('include-timestamps');

    // Model indicator (header)
    elements.modelIndicator = document.getElementById('model-indicator');
    elements.currentModelName = document.getElementById('current-model-name');
    elements.currentModelStatus = document.getElementById('current-model-status');

    // Main UI
    elements.languageSelector = document.getElementById('language-selector');
    elements.languageSelect = document.getElementById('language-select');
    elements.transcribeBtn = document.getElementById('transcribe-btn');
    elements.btnText = document.querySelector('.btn-text');
    elements.btnLoading = document.querySelector('.btn-loading');
    elements.progressContainer = document.getElementById('progress-container');
    elements.progressFill = document.getElementById('progress-fill');
    elements.progressText = document.getElementById('progress-text');
    elements.results = document.getElementById('results');
    elements.resultDuration = document.getElementById('result-duration');
    elements.resultWords = document.getElementById('result-words');
    elements.resultTime = document.getElementById('result-time');
    elements.resultLanguage = document.getElementById('result-language');
    elements.transcriptionText = document.getElementById('transcription-text');
    elements.copyBtn = document.getElementById('copy-btn');
    elements.downloadBtn = document.getElementById('download-btn');
    elements.historyList = document.getElementById('history-list');
    elements.clearHistory = document.getElementById('clear-history');
}

function initEventListeners() {
    // Tabs
    elements.tabs.forEach(tab => {
        tab.addEventListener('click', () => switchTab(tab.dataset.tab));
    });

    // File selection
    elements.fileDropZone.addEventListener('click', selectFileWithDialog);
    elements.fileDropZone.addEventListener('dragover', handleDragOver);
    elements.fileDropZone.addEventListener('dragleave', handleDragLeave);
    elements.fileDropZone.addEventListener('drop', handleDrop);
    elements.clearFile.addEventListener('click', clearFile);

    // YouTube
    elements.loadYoutubeBtn.addEventListener('click', checkYoutubeVideo);
    elements.youtubeUrl.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') checkYoutubeVideo();
    });
    elements.youtubeUrl.addEventListener('input', () => {
        elements.loadYoutubeBtn.disabled = !elements.youtubeUrl.value.trim();
        // Hide options when URL changes
        elements.youtubeOptions.classList.add('hidden');
    });
    elements.useYoutubeCaptions.addEventListener('click', selectYoutubeCaptions);
    elements.useWhisper.addEventListener('click', selectYoutubeWhisper);
    elements.clearYoutube.addEventListener('click', clearYoutube);

    // Settings
    elements.settingsBtn.addEventListener('click', openSettings);
    elements.closeSettings.addEventListener('click', closeSettings);
    elements.settingsOverlay.addEventListener('click', (e) => {
        if (e.target === elements.settingsOverlay) closeSettings();
    });
    elements.modelIndicator.addEventListener('click', openSettings);

    // Model
    elements.modelSelect.addEventListener('change', onModelChange);
    elements.downloadModelBtn.addEventListener('click', downloadModel);

    // Language (save on change)
    elements.languageSelect.addEventListener('change', () => {
        const settings = loadSettings();
        settings.preferredLanguage = elements.languageSelect.value;
        saveSettings(settings);
    });

    // Timestamps toggle (save on change)
    elements.includeTimestamps.addEventListener('change', () => {
        const settings = loadSettings();
        settings.includeTimestamps = elements.includeTimestamps.checked;
        saveSettings(settings);
    });

    // Transcribe
    elements.transcribeBtn.addEventListener('click', transcribe);

    // Results
    elements.copyBtn.addEventListener('click', copyToClipboard);
    elements.downloadBtn.addEventListener('click', downloadTranscription);

    // History
    elements.clearHistory.addEventListener('click', clearAllHistory);

    // Tauri events
    setupTauriListeners();
}

// Tab switching
function switchTab(tabName) {
    activeTab = tabName;
    elements.tabs.forEach(t => {
        t.classList.toggle('active', t.dataset.tab === tabName);
    });
    elements.tabContents.forEach(c => {
        c.classList.toggle('active', c.id === `${tabName}-tab`);
    });
    updateTranscribeButton();
}

// File handling
async function selectFileWithDialog() {
    try {
        const filePath = await open({
            multiple: false,
            filters: [{
                name: 'Audio/Video',
                extensions: ['mp3', 'wav', 'm4a', 'flac', 'ogg', 'mp4', 'avi', 'mov', 'webm', 'mkv']
            }]
        });
        if (filePath) {
            setFilePath(filePath);
        }
    } catch (error) {
        console.error('Error selecting file:', error);
    }
}

function handleDragOver(e) {
    e.preventDefault();
    elements.fileDropZone.classList.add('drag-over');
}

function handleDragLeave(e) {
    e.preventDefault();
    elements.fileDropZone.classList.remove('drag-over');
}

async function handleDrop(e) {
    e.preventDefault();
    elements.fileDropZone.classList.remove('drag-over');
    await selectFileWithDialog();
}

function setFilePath(filePath) {
    const pathParts = filePath.split(/[/\\]/);
    fileState.path = filePath;
    fileState.name = pathParts[pathParts.length - 1];

    elements.fileName.textContent = fileState.name;
    elements.fileDropZone.classList.add('hidden');
    elements.fileSelected.classList.remove('hidden');
    updateTranscribeButton();
}

function clearFile() {
    fileState.path = null;
    fileState.name = null;

    elements.fileDropZone.classList.remove('hidden');
    elements.fileSelected.classList.add('hidden');
    updateTranscribeButton();
}

// YouTube handling
function extractYoutubeVideoId(url) {
    let videoId = '';
    if (url.includes('youtu.be/')) {
        videoId = url.split('youtu.be/')[1].split('?')[0];
    } else if (url.includes('watch?v=')) {
        videoId = url.split('watch?v=')[1].split('&')[0];
    } else if (url.includes('/shorts/')) {
        videoId = url.split('/shorts/')[1].split('?')[0];
    }
    return videoId;
}

async function checkYoutubeVideo() {
    const url = elements.youtubeUrl.value.trim();
    if (!url) return;

    const youtubeRegex = /^(https?:\/\/)?(www\.)?(youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/shorts\/)[\w-]+/;
    if (!youtubeRegex.test(url)) {
        alert('Please enter a valid YouTube URL');
        return;
    }

    const videoId = extractYoutubeVideoId(url);
    if (!videoId) {
        alert('Could not extract video ID from URL');
        return;
    }

    elements.loadYoutubeBtn.textContent = 'Checking...';
    elements.loadYoutubeBtn.disabled = true;

    try {
        const result = await invoke('check_youtube_captions', { videoId });

        youtubeState.videoId = videoId;
        youtubeState.url = url;
        youtubeState.title = result.title || `Video ${videoId}`;
        youtubeState.hasCaptions = result.has_captions;
        youtubeState.captionLanguages = result.caption_languages || [];
        youtubeState.method = null; // Reset method selection

        elements.youtubeVideoTitle.textContent = youtubeState.title;
        elements.youtubeOptions.classList.remove('hidden');
        elements.youtubeInputSection.classList.add('hidden');

        if (youtubeState.hasCaptions) {
            elements.useYoutubeCaptions.disabled = false;
            // Show only the first (original) language
            const originalLang = youtubeState.captionLanguages[0] || 'unknown';
            elements.captionsStatus.textContent = `Captions available (${originalLang})`;
            elements.captionsStatus.style.color = 'var(--success)';
        } else {
            elements.useYoutubeCaptions.disabled = true;
            elements.captionsStatus.textContent = 'No captions available for this video';
            elements.captionsStatus.style.color = 'var(--warning)';
        }

        // Handle Whisper button based on yt-dlp availability
        if (ytdlpAvailable) {
            elements.useWhisper.disabled = false;
            elements.useWhisper.querySelector('.option-desc').textContent = 'More accurate, processes audio locally';
        } else {
            elements.useWhisper.disabled = true;
            elements.useWhisper.querySelector('.option-desc').textContent = 'Requires yt-dlp: brew install yt-dlp';
        }

    } catch (error) {
        console.error('Error checking YouTube video:', error);
        youtubeState.videoId = videoId;
        youtubeState.url = url;
        youtubeState.title = `Video ${videoId}`;
        youtubeState.hasCaptions = false;
        youtubeState.captionLanguages = [];

        elements.youtubeVideoTitle.textContent = youtubeState.title;
        elements.youtubeOptions.classList.remove('hidden');
        elements.youtubeInputSection.classList.add('hidden');
        elements.useYoutubeCaptions.disabled = true;

        if (ytdlpAvailable) {
            elements.captionsStatus.textContent = 'Could not check captions. You can still use Whisper.';
            elements.useWhisper.disabled = false;
        } else {
            elements.captionsStatus.textContent = 'Could not check captions. Install yt-dlp to use Whisper.';
            elements.useWhisper.disabled = true;
        }
        elements.captionsStatus.style.color = 'var(--warning)';
    } finally {
        elements.loadYoutubeBtn.textContent = 'Check';
        elements.loadYoutubeBtn.disabled = false;
    }
}

function selectYoutubeCaptions() {
    if (!youtubeState.videoId || !youtubeState.hasCaptions) return;

    youtubeState.method = 'captions';

    elements.youtubeSelectedName.textContent = youtubeState.title;
    elements.youtubeSelectedMethod.textContent = '(YouTube Captions)';
    elements.youtubeOptions.classList.add('hidden');
    elements.youtubeSelected.classList.remove('hidden');
    updateTranscribeButton();
}

function selectYoutubeWhisper() {
    if (!youtubeState.videoId) return;

    youtubeState.method = 'whisper';

    elements.youtubeSelectedName.textContent = youtubeState.title;
    elements.youtubeSelectedMethod.textContent = '(Whisper)';
    elements.youtubeOptions.classList.add('hidden');
    elements.youtubeSelected.classList.remove('hidden');
    updateTranscribeButton();
}

function clearYoutube() {
    youtubeState.url = null;
    youtubeState.videoId = null;
    youtubeState.title = null;
    youtubeState.method = null;
    youtubeState.hasCaptions = false;
    youtubeState.captionLanguages = [];

    elements.youtubeUrl.value = '';
    elements.youtubeInputSection.classList.remove('hidden');
    elements.youtubeOptions.classList.add('hidden');
    elements.youtubeSelected.classList.add('hidden');
    updateTranscribeButton();
}

// Settings handling
function openSettings() {
    settingsOpen = true;
    elements.settingsOverlay.classList.remove('hidden');
    loadDownloadedModels();
}

function closeSettings() {
    settingsOpen = false;
    elements.settingsOverlay.classList.add('hidden');
}

async function onModelChange() {
    const settings = loadSettings();
    settings.preferredModel = elements.modelSelect.value;
    saveSettings(settings);
    updateModelIndicator();
    await checkModelStatus();
}

function updateModelIndicator() {
    const model = elements.modelSelect.value;
    const modelNames = {
        'tiny': 'Tiny',
        'base': 'Base',
        'small': 'Small',
        'medium': 'Medium',
        'large-v3-turbo': 'Turbo'
    };
    elements.currentModelName.textContent = modelNames[model] || model;
}

async function loadDownloadedModels() {
    try {
        const models = await invoke('get_downloaded_models');
        if (models && models.length > 0) {
            elements.downloadedModelsList.innerHTML = models.map(m => `
                <div class="downloaded-model-item">
                    <span class="model-name">${m.name}</span>
                    <span class="model-size">${formatFileSize(m.size)}</span>
                    <button class="btn-delete-model" onclick="deleteModel('${m.name}')" title="Delete">&times;</button>
                </div>
            `).join('');
        } else {
            elements.downloadedModelsList.innerHTML = '<p class="no-models">No models downloaded yet</p>';
        }
    } catch (error) {
        console.error('Error loading downloaded models:', error);
        elements.downloadedModelsList.innerHTML = '<p class="error">Error loading models</p>';
    }
}

window.deleteModel = async function(modelName) {
    if (!confirm(`Delete ${modelName} model?`)) return;
    try {
        await invoke('delete_whisper_model', { modelName });
        await loadDownloadedModels();
        await checkModelStatus();
    } catch (error) {
        console.error('Error deleting model:', error);
        alert('Error deleting model: ' + error);
    }
};

function formatFileSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
}

// Model handling
async function checkModelStatus() {
    const model = elements.modelSelect.value;
    try {
        const downloaded = await invoke('check_model_downloaded', { modelName: model });

        if (downloaded) {
            elements.modelStatus.textContent = 'Ready';
            elements.modelStatus.className = 'model-status downloaded';
            elements.downloadModelBtn.classList.add('hidden');
            elements.currentModelStatus.classList.add('ready');
            elements.currentModelStatus.classList.remove('not-ready');
        } else {
            elements.modelStatus.textContent = 'Not downloaded';
            elements.modelStatus.className = 'model-status not-downloaded';
            elements.downloadModelBtn.classList.remove('hidden');
            elements.currentModelStatus.classList.add('not-ready');
            elements.currentModelStatus.classList.remove('ready');
        }

        updateModelIndicator();
        updateTranscribeButton();
    } catch (error) {
        console.error('Error checking model status:', error);
        elements.modelStatus.textContent = 'Error';
        elements.modelStatus.className = 'model-status';
    }
}

async function downloadModel() {
    if (isDownloading) return;

    const model = elements.modelSelect.value;
    isDownloading = true;

    // Show progress, hide button
    elements.downloadModelBtn.classList.add('hidden');
    elements.downloadProgress.classList.remove('hidden');
    elements.modelSelect.disabled = true;
    elements.downloadProgressFill.style.width = '0%';
    elements.downloadProgressText.textContent = 'Starting download...';

    try {
        await invoke('download_whisper_model', { modelName: model });

        // Download complete
        elements.downloadProgress.classList.add('hidden');
        elements.modelStatus.textContent = 'Ready';
        elements.modelStatus.className = 'model-status downloaded';

        updateTranscribeButton();
    } catch (error) {
        console.error('Download error:', error);
        elements.downloadProgress.classList.add('hidden');
        elements.downloadModelBtn.classList.remove('hidden');
        elements.modelStatus.textContent = 'Download failed';
        elements.modelStatus.className = 'model-status not-downloaded';
        alert('Error downloading model: ' + error);
    } finally {
        isDownloading = false;
        elements.modelSelect.disabled = false;
    }
}

function updateTranscribeButton() {
    // Check if we have input based on active tab
    let hasInput = false;
    let needsModel = true;

    if (activeTab === 'file') {
        hasInput = fileState.path !== null;
    } else if (activeTab === 'youtube') {
        hasInput = youtubeState.method !== null;
        needsModel = youtubeState.method !== 'captions';
    }

    const modelReady = elements.modelStatus.classList.contains('downloaded');
    elements.transcribeBtn.disabled = !hasInput || (needsModel && !modelReady) || isProcessing || isDownloading;

    // Update button text and language selector visibility based on context
    if (activeTab === 'youtube' && youtubeState.method === 'captions') {
        elements.btnText.textContent = 'Get Captions';
        elements.languageSelector.classList.add('hidden');
    } else {
        elements.btnText.textContent = 'Transcribe';
        elements.languageSelector.classList.remove('hidden');
    }
}

// Transcription
async function transcribe() {
    isProcessing = true;
    elements.btnText.classList.add('hidden');
    elements.btnLoading.classList.remove('hidden');
    elements.transcribeBtn.disabled = true;
    elements.progressContainer.classList.remove('hidden');
    elements.results.classList.add('hidden');
    elements.progressFill.style.width = '0%';

    try {
        let result;

        if (activeTab === 'youtube' && youtubeState.method) {
            if (youtubeState.method === 'captions') {
                elements.progressText.textContent = 'Fetching YouTube captions...';
                result = await invoke('get_youtube_captions', {
                    videoId: youtubeState.videoId,
                    language: elements.languageSelect.value,
                    includeTimestamps: elements.includeTimestamps.checked
                });
            } else {
                elements.progressText.textContent = 'Downloading audio from YouTube...';
                result = await invoke('transcribe_youtube', {
                    url: youtubeState.url,
                    options: {
                        model: elements.modelSelect.value,
                        language: elements.languageSelect.value,
                        include_timestamps: elements.includeTimestamps.checked
                    }
                });
            }
        } else if (activeTab === 'file' && fileState.path) {
            elements.progressText.textContent = 'Loading model...';
            result = await invoke('transcribe_file', {
                filePath: fileState.path,
                options: {
                    model: elements.modelSelect.value,
                    language: elements.languageSelect.value,
                    include_timestamps: elements.includeTimestamps.checked
                }
            });
        } else {
            throw new Error('No input selected');
        }

        showResult(result);
        await loadHistory();
    } catch (error) {
        console.error('Transcription error:', error);
        alert('Error: ' + error);
    } finally {
        isProcessing = false;
        elements.btnText.classList.remove('hidden');
        elements.btnLoading.classList.add('hidden');
        updateTranscribeButton();
        elements.progressContainer.classList.add('hidden');
    }
}

function showResult(result) {
    elements.results.classList.remove('hidden');
    elements.transcriptionText.value = result.text;

    const wordCount = result.text.split(/\s+/).filter(w => w.length > 0).length;

    elements.resultDuration.textContent = result.audio_info ? result.audio_info.duration_str : '';
    elements.resultWords.textContent = `${wordCount} words`;
    elements.resultTime.textContent = `${result.processing_time.toFixed(1)}s`;
    elements.resultLanguage.textContent = result.language || '';
}

// Clipboard and download
async function copyToClipboard() {
    const text = elements.transcriptionText.value;
    try {
        await navigator.clipboard.writeText(text);
        const originalText = elements.copyBtn.textContent;
        elements.copyBtn.textContent = 'Copied!';
        setTimeout(() => {
            elements.copyBtn.textContent = originalText;
        }, 1500);
    } catch (error) {
        console.error('Copy failed:', error);
    }
}

function downloadTranscription() {
    const text = elements.transcriptionText.value;
    const blob = new Blob([text], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    // Use current source name
    let fileName = 'transcription.txt';
    if (activeTab === 'file' && fileState.name) {
        fileName = fileState.name.replace(/\.[^.]+$/, '.txt');
    } else if (activeTab === 'youtube' && youtubeState.title) {
        fileName = youtubeState.title.replace(/[^a-zA-Z0-9]/g, '_') + '.txt';
    }
    a.download = fileName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

// History
let historyData = []; // Store history data for inline expansion

async function loadHistory() {
    try {
        historyData = await invoke('get_history');
        renderHistory(historyData);
    } catch (error) {
        console.error('Error loading history:', error);
    }
}

function renderHistory(history) {
    if (!history || history.length === 0) {
        elements.historyList.innerHTML = '<p class="history-empty">No transcriptions yet</p>';
        return;
    }

    elements.historyList.innerHTML = history.slice(0, 10).map(item => {
        const isExpanded = expandedHistoryId === item.id;
        return `
        <div class="history-item ${isExpanded ? 'expanded' : ''}" data-id="${item.id}">
            <div class="history-item-header" onclick="toggleHistoryItem('${item.id}')">
                <div class="history-item-info">
                    <div class="history-item-name">${escapeHtml(item.source_name)}</div>
                    <div class="history-item-meta">
                        ${item.audio_info ? item.audio_info.duration_str + ' · ' : ''}${item.word_count} words · ${formatDate(item.timestamp)}
                    </div>
                </div>
                <div class="history-item-actions">
                    <button class="btn-small btn-danger" onclick="handleDeleteClick(event, '${item.id}')" title="Delete">×</button>
                </div>
            </div>
            ${isExpanded ? `
            <div class="history-item-content">
                <textarea class="history-transcription" readonly>${escapeHtml(item.transcription)}</textarea>
                <div class="history-item-footer">
                    <button class="btn-small" onclick="copyHistoryText('${item.id}')">Copy</button>
                    <button class="btn-small" onclick="downloadHistoryText('${item.id}')">Save</button>
                </div>
            </div>
            ` : ''}
        </div>
    `}).join('');
}

window.toggleHistoryItem = async function(id) {
    if (expandedHistoryId === id) {
        expandedHistoryId = null;
    } else {
        expandedHistoryId = id;
    }
    renderHistory(historyData);
};

window.copyHistoryText = async function(id) {
    const item = historyData.find(h => h.id === id);
    if (item) {
        try {
            await navigator.clipboard.writeText(item.transcription);
        } catch (error) {
            console.error('Copy failed:', error);
        }
    }
};

window.downloadHistoryText = function(id) {
    const item = historyData.find(h => h.id === id);
    if (item) {
        const blob = new Blob([item.transcription], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = item.source_name.replace(/[^a-zA-Z0-9]/g, '_') + '.txt';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
};

window.handleDeleteClick = function(event, id) {
    event.stopPropagation();
    event.preventDefault();
    deleteTranscription(id);
};

window.deleteTranscription = async function(id) {
    const confirmed = await ask('Delete this transcription?', {
        title: 'Confirm Delete',
        kind: 'warning'
    });
    if (!confirmed) return;
    try {
        await invoke('delete_transcription', { id });
        if (expandedHistoryId === id) {
            expandedHistoryId = null;
        }
        await loadHistory();
    } catch (error) {
        console.error('Error deleting transcription:', error);
    }
};

async function clearAllHistory() {
    const confirmed = await ask('Delete all history?', {
        title: 'Confirm Delete',
        kind: 'warning'
    });
    if (!confirmed) return;
    try {
        await invoke('clear_history');
        await loadHistory();
        elements.results.classList.add('hidden');
    } catch (error) {
        console.error('Error clearing history:', error);
    }
}

// Tauri event listeners
function setupTauriListeners() {
    listen('transcription-progress', (event) => {
        const data = event.payload;
        if (data.progress !== undefined) {
            elements.progressFill.style.width = `${data.progress * 100}%`;
        }
        if (data.message) {
            elements.progressText.textContent = data.message;
        }
    });

    listen('model-download-progress', (event) => {
        const data = event.payload;
        elements.downloadProgressFill.style.width = `${data.progress * 100}%`;
        elements.downloadProgressText.textContent = `${data.downloaded_formatted} / ${data.total_formatted}`;
    });
}

// Utilities
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function formatDate(isoString) {
    const date = new Date(isoString);
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;

    return date.toLocaleDateString();
}
