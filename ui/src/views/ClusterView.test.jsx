

import { render, unmountComponentAtNode } from "react-dom";
import { waitFor } from "@testing-library/react";
import { screen } from '@testing-library/dom';
import ClusterView from './ClusterView';
import { act } from "react-dom/test-utils";
import { Client } from "../services/client";

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

test('renders ClusterStateView', async () => {
  const clusterState = {
      "state": {
        "seed_addrs": [],
        "node_states": {
          "node-green-uCdq/1656700092": {
            "key_values": {
              "available_services": {
                "value": "searcher",
                "version": 3
              },
              "grpc_address": {
                "value": "127.0.0.1:7281",
                "version": 2
              },
              "heartbeat": {
                "value": "24",
                "version": 27
              }
            },
            "max_version": 27
          }
        }
      },
      "live_nodes": [],
      "dead_nodes": []
  };
  Client.prototype.cluster.mockImplementation(() => Promise.resolve(clusterState));

  await act(async () => {
    render(<ClusterView />, container);
  });

  await waitFor(() => expect(screen.getByText(/node-green-uCdq/)).toBeInTheDocument());
});
