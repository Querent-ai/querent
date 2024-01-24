

import { Index, SearchRequest } from "./models";

export interface SearchComponentProps {
  searchRequest: SearchRequest;
  queryRunning: boolean;
  index: null | Index;
  onSearchRequestUpdate(searchRequest: SearchRequest): void;
  runSearch(searchRequest: SearchRequest): void;
}
