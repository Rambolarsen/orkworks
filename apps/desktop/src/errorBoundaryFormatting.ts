export interface BoundaryErrorInfo {
  componentStack?: string | null;
}

export interface FormattedBoundaryError {
  name: string;
  message: string;
  stack: string;
  componentStack: string;
}

export function formatBoundaryError(error: unknown, info: BoundaryErrorInfo): FormattedBoundaryError {
  if (error instanceof Error) {
    return {
      name: error.name || "Error",
      message: error.message,
      stack: error.stack ?? "",
      componentStack: info.componentStack ?? "",
    };
  }
  return {
    name: "Error",
    message: String(error),
    stack: "",
    componentStack: info.componentStack ?? "",
  };
}
