import type { Action } from "svelte/action";

type Edge = "left" | "right";

type Options = {
  handles?: Edge[];
  minWidth?: number;
  maxWidth?: number;
  handleSize?: number;
  visible?: boolean;
};

const DEFAULT_OPTIONS: Required<
  Pick<Options, "handles" | "minWidth" | "maxWidth" | "handleSize" | "visible">
> = {
  handles: ["right"],
  minWidth: 160,
  maxWidth: Number.POSITIVE_INFINITY,
  handleSize: 10,
  visible: true,
};

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

export const edgeResize: Action<HTMLElement, Options> = (node, opts = {}) => {
  const options = { ...DEFAULT_OPTIONS, ...opts };
  const handles: Array<{ edge: Edge; el: HTMLDivElement }> = [];

  if (getComputedStyle(node).position === "static") {
    node.style.position = "relative";
  }

  const applyWidth = (width: number) => {
    const next = clamp(width, options.minWidth, options.maxWidth);
    node.style.width = `${next}px`;
    node.style.flex = `0 0 ${next}px`;
    node.dataset.resizedWidth = String(next);
  };

  const startWidth = node.getBoundingClientRect().width;
  if (startWidth > 0) applyWidth(startWidth);

  for (const edge of options.handles) {
    const handle = document.createElement("div");
    handle.setAttribute("aria-hidden", "true");
    handle.dataset.edgeResizeHandle = edge;
    handle.style.position = "absolute";
    handle.style.top = "0";
    handle.style.bottom = "0";
    handle.style.width = `${options.handleSize}px`;
    handle.style.zIndex = "55";
    handle.style.touchAction = "none";
    handle.style.userSelect = "none";
    handle.style.cursor = "ew-resize";
    handle.style.pointerEvents = "auto";
    handle.style.background = options.visible
      ? "linear-gradient(to right, rgba(0,0,0,0.16), rgba(255,255,255,0.10), rgba(0,0,0,0.16))"
      : "transparent";
    handle.style.opacity = "0.9";
    handle.style.transition =
      "background-color 120ms ease, box-shadow 120ms ease, transform 120ms ease";
    handle.style.boxShadow =
      "inset 0 0 0 1px rgba(255,255,255,0.12), inset 0 0 0 2px rgba(0,0,0,0.22)";
    handle.style[edge] = `-${Math.floor(options.handleSize / 2)}px`;

    const setHover = (hover: boolean) => {
      handle.style.transform = hover ? "scaleX(1.05)" : "scaleX(1)";
      handle.style.boxShadow = hover
        ? "inset 0 0 0 1px rgba(255,255,255,0.26), inset 0 0 0 2px rgba(0,0,0,0.34), 0 0 0 1px rgba(255,255,255,0.08)"
        : "inset 0 0 0 1px rgba(255,255,255,0.12), inset 0 0 0 2px rgba(0,0,0,0.22)";
      handle.style.background = hover
        ? "linear-gradient(to right, rgba(0,0,0,0.22), rgba(255,255,255,0.18), rgba(0,0,0,0.22))"
        : "linear-gradient(to right, rgba(0,0,0,0.16), rgba(255,255,255,0.10), rgba(0,0,0,0.16))";
    };
    handle.addEventListener("mouseenter", () => setHover(true));
    handle.addEventListener("mouseleave", () => setHover(false));
    handle.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      event.stopPropagation();
      const rect = node.getBoundingClientRect();
      const startX = event.clientX;
      const initialWidth = rect.width;

      const onMove = (moveEvent: PointerEvent) => {
        const delta = moveEvent.clientX - startX;
        const nextWidth =
          edge === "right" ? initialWidth + delta : initialWidth - delta;
        applyWidth(nextWidth);
      };

      const onUp = () => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
        document.removeEventListener("pointercancel", onUp);
      };

      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
      document.addEventListener("pointercancel", onUp);
    });

    const indicator = document.createElement("div");
    indicator.style.position = "absolute";
    indicator.style.top = "50%";
    indicator.style.left = edge === "left" ? "2px" : "auto";
    indicator.style.right = edge === "right" ? "2px" : "auto";
    indicator.style.width = "4px";
    indicator.style.height = "56px";
    indicator.style.transform = "translateY(-50%)";
    indicator.style.borderRadius = "9999px";
    indicator.style.background = "rgba(255, 255, 255, 0.68)";
    indicator.style.boxShadow =
      "0 0 0 1px rgba(0, 0, 0, 0.55), 0 1px 2px rgba(0, 0, 0, 0.45)";
    handle.appendChild(indicator);

    node.appendChild(handle);
    handles.push({ edge, el: handle });
  }

  return {
    update(nextOpts = {}) {
      const next = { ...DEFAULT_OPTIONS, ...nextOpts };
      if (
        next.minWidth !== options.minWidth ||
        next.maxWidth !== options.maxWidth
      ) {
        const current = node.getBoundingClientRect().width;
        applyWidth(current);
      }
      // lightweight update path; re-rendering handles isn't needed for current use
      Object.assign(options, next);
    },
    destroy() {
      for (const { el } of handles) el.remove();
    },
  };
};
