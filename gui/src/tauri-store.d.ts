declare module '@tauri-apps/plugin-store' {
  export class Store {
    constructor(filename: string);
    get(key: string): Promise<any>;
    set(key: string, value: any): Promise<void>;
    save(): Promise<void>;
    load(): Promise<void>;
  }
}