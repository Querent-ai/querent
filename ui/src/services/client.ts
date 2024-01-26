

import { Cluster, QuesterBuildInfo, SemanticServiceCounters } from "../utils/models";

export class Client {
  private readonly _host: string

  constructor(host?: string) {
    if (!host) {
      this._host = window.location.origin
    } else {
      this._host = host
    }
  }

  apiRoot(): string {
    return this._host + "/api/v1/";
  }

  async cluster(): Promise<Cluster> {
    return await this.fetch(`${this.apiRoot()}cluster`, this.defaultGetRequestParams());
  }

  async buildInfo(): Promise<QuesterBuildInfo> {
    return await this.fetch(`${this.apiRoot()}version`, this.defaultGetRequestParams());
  }

  // eslint-disable-next-line
  async config(): Promise<Record<string, any>> {
    return await this.fetch(`${this.apiRoot()}config`, this.defaultGetRequestParams());
  }

  async getSemanticServiceCounters(): Promise<SemanticServiceCounters> {
    return await this.fetch(`${this.apiRoot()}semantics`, this.defaultGetRequestParams());
  }

  async fetch<T>(url: string, params: RequestInit): Promise<T> {
    const response = await fetch(url, params);
    if (response.ok) {
      return response.json() as Promise<T>;
    }
    const message = await response.text();
    return await Promise.reject({
      message: message,
      status: response.status
    });
  }

  private defaultGetRequestParams(): RequestInit {
    return {
      method: "GET",
      headers: { Accept: "application/json" },
      mode: "no-cors",
      cache: "default",
    }
  }
}
