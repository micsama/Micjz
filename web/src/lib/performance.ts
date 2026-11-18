import { tick } from "svelte";

export type BootKind = "startup" | "login" | "retry";
type Kind = BootKind | "resume";
type Outcome = "ready" | "login" | "failed";
type Stage = "catalog_ms" | "settings_ms" | "entries_ms" | "balances_ms";

function timing(entry: PerformanceResourceTiming, origin: number) {
  return {
    start_ms: Math.round(entry.startTime - origin),
    duration_ms: Math.round(entry.duration),
    ttfb_ms: entry.responseStart ? Math.round(entry.responseStart - entry.startTime) : undefined,
    download_ms: entry.responseStart ? Math.round(entry.responseEnd - entry.responseStart) : undefined,
    transfer_bytes: entry.transferSize,
    encoded_bytes: entry.encodedBodySize,
    decoded_bytes: entry.decodedBodySize,
  };
}

function resourceKind(entry: PerformanceResourceTiming) {
  const path = new URL(entry.name).pathname;
  if (path.endsWith(".js")) return "js";
  if (path.endsWith(".css")) return "css";
  const endpoint = path.match(/\/api\/(catalog|settings|balances|entries)$/)?.[1];
  return endpoint;
}

async function afterPaint(started: number): Promise<number | undefined> {
  await tick();
  if (document.visibilityState !== "visible") return;
  return new Promise((resolve) => {
    const timeout = window.setTimeout(() => resolve(undefined), 500);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        clearTimeout(timeout);
        resolve(document.visibilityState === "visible" ? Math.round(performance.now() - started) : undefined);
      });
    });
  });
}

export function webPerformance(kind: Kind) {
  const began = performance.now();
  const started = kind === "startup" ? 0 : began;
  const stages: Partial<Record<Stage, number>> = {};

  return {
    async measure<T>(stage: Stage, run: () => Promise<T>): Promise<T> {
      const at = performance.now();
      try {
        return await run();
      } finally {
        stages[stage] = Math.round(performance.now() - at);
      }
    },
    async report(outcome: Outcome) {
      const total_ms = Math.round(performance.now() - started);
      const cutoff = performance.now();
      const paint_ms = outcome === "ready" ? await afterPaint(started) : undefined;
      const navigation = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
      const resources = (performance.getEntriesByType("resource") as PerformanceResourceTiming[])
        .filter((entry) => entry.startTime >= started && entry.responseEnd <= cutoff)
        .flatMap((entry) => {
          const label = resourceKind(entry);
          return label ? [{ label, ...timing(entry, started) }] : [];
        });
      const fcp = performance.getEntriesByName("first-contentful-paint", "paint")[0];
      void fetch("api/web-performance", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind,
          outcome,
          standalone: matchMedia("(display-mode: standalone)").matches || (navigator as Navigator & { standalone?: boolean }).standalone === true,
          total_ms,
          paint_ms,
          before_boot_ms: kind === "startup" ? Math.round(began) : undefined,
          first_contentful_paint_ms: kind === "startup" && fcp ? Math.round(fcp.startTime) : undefined,
          navigation:
            kind === "startup" && navigation
              ? {
                  ...timing(navigation, 0),
                  duration_ms: Math.round(navigation.responseEnd),
                  dns_ms: Math.round(navigation.domainLookupEnd - navigation.domainLookupStart),
                  connect_ms: Math.round(navigation.connectEnd - navigation.connectStart),
                  server_wait_ms: navigation.responseStart ? Math.round(navigation.responseStart - navigation.requestStart) : undefined,
                }
              : undefined,
          resource_count: resources.length,
          resources: resources.slice(0, 24),
          ...stages,
        }),
        keepalive: true,
      }).catch(() => {});
    },
  };
}
