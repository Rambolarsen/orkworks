// Renderer-owned presentation contract; no generated text establishes check status.
import { ApiError } from './requestError.ts';
export const unverifiedAdviceWarning = 'Checks remain unverified. Run the proposed validation before relying on it.';
export type AdviceMetadata = { protocolVersion?: 3; verificationStatus?: 'unverified' };

const object = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : undefined;

export function isUnverifiedAdvice(value: unknown): boolean {
  const item = object(value);
  const reference = object(object(item?.assessment)?.improvementReference);
  const historyPurpose = object(object(item?.data)?.inputManifest)?.purpose;
  return item?.producerVersion === 3 || item?.improvementVersion === 3 || reference?.version === 3 || historyPurpose === 'brain-improvement-v3';
}

export function adviceIsSupported(value: unknown): boolean {
  const item = object(value);
  if (!item) return false;
  if (item.producerVersion !== undefined && item.producerVersion !== 3) return false;
  if (item.improvementVersion !== undefined && item.improvementVersion !== 3) return false;
  const historyPurpose = object(object(item.data)?.inputManifest)?.purpose;
  if (typeof historyPurpose === 'string' && historyPurpose.startsWith('brain-improvement-')
    && !['brain-improvement-v1','brain-improvement-v2','brain-improvement-v3'].includes(historyPurpose)) return false;
  if (historyPurpose === 'brain-improvement-v3' && item.improvementVersion !== 3) return false;
  const reference = object(item.assessment)?.improvementReference;
  if (reference !== undefined) {
    const ref = object(reference);
    if (!ref || Object.keys(ref).sort().join(',') !== 'intent,version'
      || ![1, 2, 3].includes(ref.version as number)
      || !['implementation', 'investigation'].includes(ref.intent as string)) return false;
  }
  return isUnverifiedAdvice(item)
    ? item.protocolVersion === 3 && item.verificationStatus === 'unverified'
    : item.verificationStatus === undefined;
}

export function requireSupportedAdvice(value: unknown): void {
  if (!adviceIsSupported(value)) throw Error('Unsupported or unlabelled unverified advice protocol');
}

export function requireAdviceProtocol(value: unknown): Record<string, unknown> {
  const item = object(value);
  if (item?.protocolVersion !== 3) throw Error('Advice protocol upgrade required');
  return item;
}

export function parseAdviceResponse(value: unknown): Record<string, unknown> {
  const root = requireAdviceProtocol(value);
  if (root.requestError !== undefined) {
    const error = object(root.requestError);
    if (!error || Object.keys(error).join(',') !== 'status' || !Number.isInteger(error.status)
      || (error.status as number) < 400 || (error.status as number) > 599) throw Error('Invalid advice error response');
    throw new ApiError(error.status === 426 ? 'Advice protocol upgrade required' : `Taskmaster request failed (${error.status})`, error.status as number);
  }
  const visit = (value: unknown) => {
    if (Array.isArray(value)) { value.forEach(visit); return; }
    const item = object(value);
    if (!item) return;
    requireSupportedAdvice(item);
    Object.values(item).forEach(visit);
  };
  visit(root);
  return root;
}
