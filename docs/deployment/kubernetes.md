---
title: Kubernetes (Helm)
sidebar_position: 2
---

[Helm](https://helm.sh) is a package manager for Kubernetes that allows you to configure, install, and upgrade containerized applications in a Kubernetes cluster in a version-controlled and reproducible way.

## Install Querent using Helm

Install Querent on Kubernetes with the official Querent Helm chart. If you encounter any problem with the chart, please, open an issue in our [GitHub repository](https://github.com/querent-ai/helm-charts).

## Requirements

To deploy Querent on Kubernetes, you will need:

- kubectl, compatible with your cluster (+/- 1 minor release from your cluster) (`kubectl version`)
- Helm v3 (`helm version`)
- A Kubernetes cluster

1. Install `kubectl` and `helm`

2. To install `kubectl` and `helm` locally, follow the [Kubernetes](https://kubernetes.io/docs/tasks/tools/#install-kubectl) and [Helm](https://helm.sh/docs/intro/install/) documentation pages.

3. Add the Querent Helm chart repository to Helm `helm repo add querent https://helm.querent.xyz`
4. Update the repository `helm repo update`.

5. Create and customize your configuration file `values.yaml`. Note: You can inspect the default configuration values of the chart using the   following command: `helm show values querent/querent`.

6. Deploy Querent: `helm install <deployment name> querent/querent -f values.yaml
`.

7. Check that Querent is running

It might take some time for the cluster to start. During the startup process individual pods might restart themselves several times. The command on the previous step will print the instructions on how to connect to the cluster. This endpoint can be used to access the querent search UI, as well execute standard API commands against.

## Uninstall the deployment

Run the following Helm command to uninstall the deployment: `helm uninstall <deployment name>`.
