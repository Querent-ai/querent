

import { render, unmountComponentAtNode } from "react-dom";
import {screen} from '@testing-library/dom'
import IndexesView from './IndexesView';
import { act } from "react-dom/test-utils";
import {Client} from "../services/client";

jest.mock('../services/client');
const mockedUsedNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useNavigate: () => mockedUsedNavigate,
}));

let container = null;
beforeEach(() => {
  // setup a DOM element as a render target
  container = document.createElement("div");
  document.body.appendChild(container);
});

afterEach(() => {
  // cleanup on exiting
  unmountComponentAtNode(container);
  container.remove();
  container = null;
});

test('renders IndexesView', async () => {
  const indexes = [{
    index_config: {
      index_id: 'my-new-fresh-index',
      index_uri: 'my-uri',
      indexing_settings: {
        timestamp_field: 'timestamp'
      },
      search_settings: {},
      doc_mapping: {
        store: false,
        field_mappings: [],
        tag_fields: [],
        dynamic_mapping: false,
      },
    },
    sources: [],
    create_timestamp: 1000,
    update_timestamp: 1000,
  }];
  Client.prototype.listIndexes.mockResolvedValueOnce(() => indexes);

  await act(async () => {
    render(<IndexesView />, container);
  });

  expect(screen.getByText(indexes[0].index_config.index_id)).toBeInTheDocument();
});
