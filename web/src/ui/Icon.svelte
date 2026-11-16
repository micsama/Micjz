<script module lang="ts">
  export type IconName =
    | "chevron-right"
    | "chevron-left"
    | "chevron-down"
    | "check"
    | "xmark"
    | "list"
    | "gear"
    | "trash"
    | "photo"
    | "sparkles"
    | "wallet"
    | "copy"
    | "pencil"
    | "plus"
    | "yen"
    | "arrows"
    | "grid"
    | "card"
    | "store"
    | "clock"
    | "tag"
    | "note"
    | "calendar"
    | "target"
    | "update"
    | "receipt"
    | "eye"
    | "eye-slash"
    | "chart"
    | "transfer"
    | "scale"
    | "arrow-out"
    | "arrow-in"
    | "arrow-up"
    | "arrow-down"
    | "minus";
</script>

<script lang="ts">
  // 仿 SF Symbols 的线条图标。
  let { name, size = 17, weight = 2 }: { name: IconName; size?: number | string; weight?: number } = $props();
  const paths: Record<IconName, string> = {
    "chevron-right": "M9 5l7 7-7 7",
    "chevron-left": "M15 5l-7 7 7 7",
    "chevron-down": "M5 9l7 7 7-7",
    check: "M4.5 12.5l5 5 10-11",
    xmark: "M6 6l12 12M18 6L6 18",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    gear: "M12 15.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7zM19.4 15a1.7 1.7 0 00.3 1.8l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.8-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1.1-1.5 1.7 1.7 0 00-1.8.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.8 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.5-1.1 1.7 1.7 0 00-.3-1.8l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.8.3H9a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.5 1.7 1.7 0 001.8-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.8V9a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z",
    trash: "M4 7h16M10 11v6M14 11v6M5 7l1 12a2 2 0 002 2h8a2 2 0 002-2l1-12M9 7V4h6v3",
    photo: "M4 5h16v14H4zM4 16l5-5 4 4 2-2 5 5M15.5 9.5h.01",
    wallet: "M4 7.5A2.5 2.5 0 016.5 5H18v3M4 7.5V17a2 2 0 002 2h14V8H6.5A2.5 2.5 0 014 7.5zM16 13.5h.01",
    copy: "M9 8h9a1 1 0 011 1v11a1 1 0 01-1 1H9a1 1 0 01-1-1V9a1 1 0 011-1zM16 8V5a1 1 0 00-1-1H6a1 1 0 00-1 1v11a1 1 0 001 1h2",
    pencil: "M4 20h4L19 9l-4-4L4 16v4zM13 7l4 4",
    plus: "M12 5v14M5 12h14",
    yen: "M7 4l5 7 5-7M12 11v9M8 13h8M8 16.5h8",
    arrows: "M4 8h15M15 4l4 4-4 4M20 16H5M9 12l-4 4 4 4",
    grid: "M5 4h4a1 1 0 011 1v4a1 1 0 01-1 1H5a1 1 0 01-1-1V5a1 1 0 011-1zM15 4h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1V5a1 1 0 011-1zM5 14h4a1 1 0 011 1v4a1 1 0 01-1 1H5a1 1 0 01-1-1v-4a1 1 0 011-1zM15 14h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4a1 1 0 011-1z",
    card: "M5 5h14a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2zM3 10h18M7 15h4",
    store: "M4 9l1.5-5h13L20 9M5 9v11h14V9M4 9a2.7 2.7 0 005.3 0 2.7 2.7 0 005.4 0 2.7 2.7 0 005.3 0M10 20v-5h4v5",
    clock: "M12 21a9 9 0 100-18 9 9 0 000 18zM12 7v5l3 2",
    tag: "M3.5 12.5V4.5a1 1 0 011-1h8l8.5 8.5-9 9zM8 8h.01",
    note: "M7 3h8l4 4v13a1 1 0 01-1 1H7a1 1 0 01-1-1V4a1 1 0 011-1zM14 3v5h5M9 12h7M9 16h5",
    calendar: "M5 5h14a1 1 0 011 1v13a1 1 0 01-1 1H5a1 1 0 01-1-1V6a1 1 0 011-1zM4 10h16M8 3v4M16 3v4",
    target: "M12 21a9 9 0 100-18 9 9 0 000 18zM12 16a4 4 0 100-8 4 4 0 000 8zM12 12h.01",
    update: "M12 21a9 9 0 100-18 9 9 0 000 18zM12 7v9M8 12l4 4 4-4",
    eye: "M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12zM12 15a3 3 0 100-6 3 3 0 000 6z",
    "eye-slash":
      "M3 3l18 18M10.6 5.6a9 9 0 011.4-.1c6.4 0 10 6.5 10 6.5a17 17 0 01-3 3.6M6.6 6.7C3.7 8.4 2 12 2 12s3.6 6.5 10 6.5c1.9 0 3.6-.6 5-1.4M9.9 9.9a3 3 0 004.2 4.2",
    receipt: "M6 3h12v18l-3-2-3 2-3-2-3 2zM9 8h6M9 12h6M9 16h3",
    chart: "M5 20V11M10 20V5M15 20v-6M20 20V9",
    transfer: "M4 8h15M15 4l4 4-4 4M20 16H5M9 12l-4 4 4 4",
    scale: "M12 4v16M8 20h8M5 7h14M5 7l-3 6a3 3 0 006 0zM19 7l-3 6a3 3 0 006 0z",
    "arrow-out": "M7 7l10 10M17 9v8H9",
    "arrow-in": "M17 17L7 7M7 15V7h8",
    "arrow-up": "M12 19V5M6 11l6-6 6 6",
    "arrow-down": "M12 5v14M6 13l6 6 6-6",
    minus: "M5 12h14",
    sparkles: "M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={weight}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
</svg>
