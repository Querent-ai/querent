

import { render, unmountComponentAtNode } from "react-dom";
import { waitFor } from "@testing-library/react";
import { screen } from '@testing-library/dom';
import { act } from "react-dom/test-utils";
import { Client } from "../services/client";
import NodeInfoView from "./NodeInfoView";

jest.mock('../services/client');
const mockedUsedNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useParams: () => ({
    indexId: 'my-new-fresh-index-id'
  })
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

test('renders NodeInfoView', async () => {
  const cluster = {
    cluster_id: 'my cluster id',
  };
  Client.prototype.cluster.mockImplementation(() => Promise.resolve(cluster));

  const config = {
    node_id: 'my-node-id',
  };
  Client.prototype.config.mockImplementation(() => Promise.resolve(config));

  const buildInfo = {
    version: '0.3.2',
  };
  Client.prototype.buildInfo.mockImplementation(() => Promise.resolve(buildInfo));
  await act(async () => {
    render(<NodeInfoView />, container);
  });

  await waitFor(() => expect(screen.getByText(/my-node-id/)).toBeInTheDocument());
});
