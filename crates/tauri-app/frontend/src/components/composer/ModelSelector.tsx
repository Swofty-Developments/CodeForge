import { createSignal, For, Show, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";

// Known aliases from `claude --help`: "Provide an alias for the latest model
// (e.g. 'sonnet' or 'opus') or a model's full name (e.g. 'claude-sonnet-4-6')."
// These are suggestions, not exhaustive — users can type any model ID.
const PRESETS = [
  { value: null, label: "Default", desc: "CLI default model" },
  { value: "claude-opus-4-6", label: "Opus 4.6", desc: "Most capable, 1M context" },
  { value: "claude-sonnet-4-6", label: "Sonnet 4.6", desc: "Fast + capable, 1M context" },
  { value: "claude-opus-4-0", label: "Opus 4", desc: "Previous Opus" },
  { value: "claude-sonnet-4-5", label: "Sonnet 4.5", desc: "Previous Sonnet" },
  { value: "claude-haiku-4-5", label: "Haiku 4.5", desc: "Fast and lightweight" },
];

export function ModelSelector() {
  const { store, setStore } = appStore;
  const [open, setOpen] = createSignal(false);
  const [customInput, setCustomInput] = createSignal("");
  let dropdownRef: HTMLDivElement | undefined;

  const currentLabel = () => {
    if (!store.selectedModel) return "Default";
    const preset = PRESETS.find((m) => m.value === store.selectedModel);
    return preset ? preset.label : store.selectedModel;
  };

  /** Show the confirmed model from the SDK, shortened for display. */
  const confirmedLabel = () => {
    if (!store.activeModel) return null;
    const selected = store.selectedModel;
    if (store.activeModel === selected) return null;
    // Shorten "claude-opus-4-6[1m]" → "opus-4.6"
    let m = store.activeModel;
    m = m.replace("claude-", "").replace("[1m]", "").replace("[200k]", "");
    return m;
  };

  function select(value: string | null) {
    setStore("selectedModel", value);
    setOpen(false);
    setCustomInput("");
  }

  function submitCustom() {
    const v = customInput().trim();
    if (v) {
      setStore("selectedModel", v);
      setOpen(false);
      setCustomInput("");
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
      setOpen(false);
    }
  }

  function toggle() {
    const willOpen = !open();
    setOpen(willOpen);
    if (willOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    } else {
      document.removeEventListener("mousedown", handleClickOutside);
    }
  }

  onCleanup(() => {
    document.removeEventListener("mousedown", handleClickOutside);
  });

  return (
    <div class="model-selector" ref={dropdownRef}>
      <button class="meta-pill" onClick={toggle}>
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2L2 7l10 5 10-5-10-5z" />
          <path d="M2 17l10 5 10-5" />
          <path d="M2 12l10 5 10-5" />
        </svg>
        {currentLabel()}
        <Show when={confirmedLabel()}>
          <span class="model-confirmed">{confirmedLabel()}</span>
        </Show>
        <svg class="chevron" width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><polyline points="6 9 12 15 18 9" /></svg>
      </button>

      <Show when={open()}>
        <div class="model-dropdown">
          <For each={PRESETS}>
            {(model) => {
              const isSelected = () => store.selectedModel === model.value;
              return (
                <button
                  class="model-option"
                  classList={{ selected: isSelected() }}
                  onClick={() => select(model.value)}
                >
                  <div class="model-option-header">
                    <span class="model-option-label">{model.label}</span>
                    <Show when={isSelected()}>
                      <svg class="model-check" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                    </Show>
                  </div>
                  <span class="model-option-desc">{model.desc}</span>
                </button>
              );
            }}
          </For>

          {/* Custom model input */}
          <div class="model-custom">
            <input
              class="model-custom-input"
              placeholder="or type model ID…"
              value={customInput()}
              onInput={(e) => setCustomInput(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") { e.preventDefault(); submitCustom(); }
                if (e.key === "Escape") setOpen(false);
              }}
            />
            <Show when={customInput().trim()}>
              <button class="model-custom-go" onClick={submitCustom}>Use</button>
            </Show>
          </div>

          {/* Show current custom model if not a preset */}
          <Show when={store.selectedModel && !PRESETS.some((p) => p.value === store.selectedModel)}>
            <div class="model-current-custom">
              <span class="model-option-label">
                <svg class="model-check" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--primary)" stroke-width="3" stroke-linecap="round"><polyline points="20 6 9 17 4 12" /></svg>
                {store.selectedModel}
              </span>
              <button class="model-custom-clear" onClick={() => select(null)}>Clear</button>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("model-selector-styles")) {
  const style = document.createElement("style");
  style.id = "model-selector-styles";
  style.textContent = `
    .model-selector {
      position: relative;
    }
    .model-dropdown {
      position: absolute;
      bottom: calc(100% + 6px);
      left: 0;
      min-width: 220px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      padding: 4px;
      z-index: 100;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
      animation: model-dropdown-in 120ms cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    @keyframes model-dropdown-in {
      from { opacity: 0; transform: translateY(4px) scale(0.97); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    .model-option {
      display: flex;
      flex-direction: column;
      gap: 2px;
      width: 100%;
      padding: 8px 10px;
      border-radius: var(--radius-sm);
      background: none;
      border: none;
      cursor: pointer;
      text-align: left;
      transition: background 0.12s;
    }
    .model-option:hover { background: var(--bg-accent); }
    .model-option.selected { background: rgba(107, 124, 255, 0.08); }
    .model-option-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
    }
    .model-option-label {
      font-size: 12px;
      font-weight: 500;
      color: var(--text);
      font-family: var(--font-mono);
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .model-option.selected .model-option-label { color: var(--primary); }
    .model-check { color: var(--primary); flex-shrink: 0; }
    .model-option-desc {
      font-size: 10px;
      color: var(--text-tertiary);
      font-family: var(--font-body);
      line-height: 1.3;
    }
    .model-custom {
      display: flex;
      gap: 4px;
      padding: 4px;
      margin-top: 2px;
      border-top: 1px solid var(--border);
    }
    .model-custom-input {
      flex: 1;
      font-size: 11px;
      padding: 5px 8px;
      background: var(--bg-base);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      font-family: var(--font-mono);
      outline: none;
    }
    .model-custom-input:focus { border-color: var(--primary); }
    .model-custom-input::placeholder { color: var(--text-tertiary); font-family: var(--font-body); }
    .model-custom-go {
      font-size: 10px;
      font-weight: 600;
      padding: 4px 8px;
      background: var(--primary);
      color: white;
      border-radius: var(--radius-sm);
      white-space: nowrap;
    }
    .model-current-custom {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 6px 10px;
      border-top: 1px solid var(--border);
      margin-top: 2px;
    }
    .model-custom-clear {
      font-size: 10px;
      color: var(--text-tertiary);
      padding: 2px 6px;
      border-radius: var(--radius-sm);
    }
    .model-custom-clear:hover { color: var(--text-secondary); background: var(--bg-hover); }
    .model-confirmed {
      font-size: 9px;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      opacity: 0.7;
    }
  `;
  document.head.appendChild(style);
}
