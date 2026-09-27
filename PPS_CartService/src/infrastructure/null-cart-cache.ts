import type { CartCache } from "../ports.js";

export class NullCartCache implements CartCache {
  public get(): Promise<null> { return Promise.resolve(null); }
  public set(): Promise<void> { return Promise.resolve(); }
  public delete(): Promise<void> { return Promise.resolve(); }
}
