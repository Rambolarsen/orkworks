import { useEffect, useRef, useState } from "react";
import { nextRelativeTimeDeadlineMs } from "./labels";

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

    const nextDeadline = nextRelativeTimeDeadlineMs(deadlineRef.current, nextRefresh, now.getTime());
    if (nextDeadline === deadlineRef.current) {
      return;
    }
    if (nextDeadline === null) {
      return;
    }

    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }

    deadlineRef.current = nextDeadline;
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      deadlineRef.current = null;
      setNow(new Date());
    }, nextDeadline - now.getTime());
  }, [computeNextRefresh, now]);

  useEffect(() => () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
  }, []);

  return now;
}
