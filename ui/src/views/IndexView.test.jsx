

import { unmountComponentAtNode } from "react-dom";
import { render, waitFor, screen } from "@testing-library/react";
import { act } from "react-dom/test-utils";
import { Client } from "../services/client";
import IndexView from "./IndexView";
import { BrowserRouter } from "react-router-dom";

jest.mock('../services/client');
const mockedUsedNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useParams: () => ({
    indexId: 'my-new-fresh-index-id'
  })
}));

test('renders IndexView', async () => {
  const index = {
    metadata: {
      index_config: {
        index_uri: 'my-new-fresh-index-uri',
      }
    },
    splits: []
  };
  Client.prototype.getIndex.mockImplementation(() => Promise.resolve(index));

  await act(async () => {
    render( <IndexView /> , {wrapper: BrowserRouter});
  });

  await waitFor(() => expect(screen.getByText(/my-new-fresh-index-uri/)).toBeInTheDocument());
});
