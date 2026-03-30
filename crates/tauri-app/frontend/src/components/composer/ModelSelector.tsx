import { createSignal, For, Show, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";

const MODELS = [
  { value: null, label: "Default", desc: "Use CLI default model" },
  { value: "opus", label: "Opus", desc: "Most capable, complex tasks" },
  { value: "sonnet", label: "Sonnet", desc: "Balanced speed and quality" },
  { value: "haiku", label: "Haiku", desc: "Fast and lightweight" },
] as const;

export function ModelSelector() {
  const { store, setStore } = appStore;
  const [open, setOpen] = createSignal(false);
  let dropdownRef: HTMLDivElement | undefined;

  const currentLabel = () => {
    const m = MODELS.find((m) => m.value === store.selectedModel);
    return m ? m.label : "Default";
  };

  function select(value: string | null) {
    setStore("selectedModel", value);
    setOpen(false);
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
        <svg class="chevron" width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><polyline points="6 9 12 15 18 9" /></svg>
      </button>

      <Show when={open()}>
        <div class="model-dropdown">
          <For each={MODELS}>
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
      min-width: 200px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      padding: 4px;
      z-index: 100;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
      animation: model-dropdown-in 120ms cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    @keyframes model-dropdown-in {
      from {
        opacity: 0;
        transform: translateY(4px) scale(0.97);
      }
      to {
        opacity: 1;
        transform: translateY(0) scale(1);
      }
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
    .model-option:hover {
      background: var(--bg-accent);
    }
    .model-option.selected {
      background: rgba(107, 124, 255, 0.08);
    }
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
      font-family: var(--font-body);
    }
    .model-option.selected .model-option-label {
      color: var(--primary);
    }
    .model-check {
      color: var(--primary);
      flex-shrink: 0;
    }
    .model-option-desc {
      font-size: 10px;
      color: var(--text-tertiary);
      font-family: var(--font-body);
      line-height: 1.3;
    }
  `;
  document.head.appendChild(style);
}
