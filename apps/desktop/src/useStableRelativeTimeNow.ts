import { useEffect, useRef, useState } from "react";
import { shouldRetainRelativeTimeRefresh } from "./labels";

export function useStableRelativeTimeNow(
  computeNextRefresh: (now: Date) => number | null,
): Date {
  const [now, setNow] = useState(() => new Date());
  const timerRef = useRef<number | null>(null);
  const deadlineRef = useRef<number | null>(null);

  useEffect(() => {
    const nextRefresh = computeNextRefresh(now);

    if (nextRefresh === null) {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
        deadlineRef.current = null;
      }
      return;
    }

    if (shouldRetainRelativeTimeRefresh(deadlineRef.current, nextRefresh, now.getTime())) {
      return;
    }

    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }

    deadlineRef.current = now.getTime() + nextRefresh;
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      deadlineRef.current = null;
      setNow(new Date());
    }, nextRefresh);
  }, [computeNextRefresh, now]);

  useEffect(() => () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
  }, []);

  return now;
}
