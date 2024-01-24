

import { Cluster, Index, IndexMetadata, QuesterBuildInfo, SearchRequest, SearchResponse, SplitMetadata } from "../utils/models";
import { serializeSortByField } from "../utils/urls";

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

  async search(request: SearchRequest): Promise<SearchResponse> {
    // TODO: improve validation of request.
    if (request.indexId === null || request.indexId === undefined) {
      throw Error("Search request must have and index id.")
    }
    const url = this.buildSearchUrl(request);
    return this.fetch(url.toString(), this.defaultGetRequestParams());
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
  //
  // Index management API
  //
  async getIndex(indexId: string): Promise<Index> {
    const [metadata, splits] = await Promise.all([
      this.getIndexMetadata(indexId),
      this.getAllSplits(indexId)
    ]);
    return {
      metadata: metadata,
      splits: splits
    }
  }

  async getIndexMetadata(indexId: string): Promise<IndexMetadata> {
    return this.fetch(`${this.apiRoot()}indexes/${indexId}`, {});
  }

  async getAllSplits(indexId: string): Promise<Array<SplitMetadata>> {
    // TODO: restrieve all the splits.
    const results: {splits: Array<SplitMetadata>} = await this.fetch(`${this.apiRoot()}indexes/${indexId}/splits?limit=10000`, {});

    return results['splits'];
  }

  async listIndexes(): Promise<Array<IndexMetadata>> {
    return this.fetch(`${this.apiRoot()}indexes`, {});
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

  buildSearchUrl(request: SearchRequest): URL {
    const url: URL = new URL(`${request.indexId}/search`, this.apiRoot());
    // TODO: the trim should be done in the backend.
    url.searchParams.append("query", request.query.trim() || "*");
    url.searchParams.append("max_hits", "20");
    if (request.startTimestamp) {
      url.searchParams.append(
        "start_timestamp",
        request.startTimestamp.toString()
      );
    }
    if (request.endTimestamp) {
      url.searchParams.append(
        "end_timestamp",
        request.endTimestamp.toString()
      );
    }
    if (request.sortByField) {
      url.searchParams.append(
        "sort_by_field",
        serializeSortByField(request.sortByField)
      );
    }
    return url;
  }
}
