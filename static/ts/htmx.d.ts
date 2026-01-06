/**
 * HTMX type declarations
 */

declare const htmx: {
  process(element: Element): void;
  ajax(method: string, url: string, options: {
    target: string;
    swap: string;
    values?: Record<string, string>;
  }): Promise<void>;
};
