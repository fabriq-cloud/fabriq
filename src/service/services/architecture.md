# Architectural

## Definitions

- Host: Something that accepts `Workloads` declaratively as a unit (can be a cluster or just a single CPU embedded device).
- Workload: A set of declarative resources deployed as a unit.
- Target: A set of rules that match `Workloads` to `Hosts`.
- Template: A declarative template that is filled with values for a `Workload`'s deployment.
- Deployment: A specific `Deployment` instance of a `Workload` with its own `Template` and `Target`.
- Assignment: A record of the current assignment of a `Deployment` to a `Host` via its `Target` rules (when there is a limited number of `replicas` required in the `Deployment`)

## Life of Change

- Proposed change is submitted to service
  - Service validates change
  - Service enqueues execution of change as event
- Service continously processes change feed
  - Pulls change
  - Commits change to persistence
  - All applied processors process change for downstream updates (gitops, messaging).

## HTTP Endpoint Approach

- Query host
  - Query targets for Host Labels
    - Query deployments for Targets
      - Query workloads for Deployments
        - Query template for Workloads
          - Add templates updated_at >= incoming timestamp to change list
      - Add workloads by updated_at >= incoming timestamp OR has template in change list to change list
    - Add deployments by updated_at >= incoming timestamp (but with no deleted_at) OR has workload in change list to change list
    - Add tombstones for deployments for deleted_at >= incoming timestamp so they are deleted on host
    - Render all deployments using the underlying workload and templates.
    - Return to host

## Messaging Based Approach

- Maintain topics for all nouns in the system plus special HostDeployment and RenderedDeployments topics. Enables both disconnected delivery to `Hosts` and an audit stream on what happened.

- `Host`, `Deployment`, `Target`, `Template`, `Workload`, `Workspace` topics: UPDATE AND DELETE messages with previous and new state of object.

- `RenderedDeployment` topics: Contains ordered UPDATE and DELETE messages for each `Deployments` with complete updated declarative state of deployment.

- HostDeployment topics: Contains UPDATE and DELETE messages for Deployments for a particular Host (based on underlying changes to `Hosts`, `Targets`, and `Deployments`) that point to the topics it should watch.

- Reconciliation Loop for Host
  - Host processes all messages in its Host queue to get to the spanning set of deployments it should be deploying
  - For any deployments that are DELETE after this processing, it deletes all of the current assets as part of the deployment.
  - For all deployments that exist, it confirms subscription to that deployment topic.
  - For all deployment topics, it pulls messages and applies them, updating its bookmark.
  - All messages are idempotent, such that at least once messaging semantics can be used.

## Service

- Create a X:

  - Create in DB
  - Send message that X created

- Update X

  - Update in DB
  - Send message that X updated

- Delete X

  - Delete in DB
  - Send message that X deleted

## Process

Process all message types in order

- Receive change
-

- Update a target

  - Build list current hosts with existing target
  - Build list hosts with new target
  - Delta the two lists
  - Send DELETE messages to the current hosts that were removed to the respective host's topic
  - Send CREATE messages to teh new hosts that were added to the respective host's topic

- Create/update a template

  - Sends updated template on Template topic
  - Query all of the deployments using template
  - For all deployments, send an UPDATE message over HostUpdates topic with the rendered deployment using the template.

- Create a deployment

  - Create in DB
  - Send a CREATE message for all hosts matching deployment (through target) on each host's topic
  - Create a deployment topic
  - Send an UPDATE message with the rendered deployment (using template).

- Delete a host:

  - Delete in DB
  - Send a DELETE message for deployments matching host (through target) on host topic

- Delete a deployment

  - Send a DELETE message for all hosts matching deployment (through target) on each host's topic
  - Delete in DB

## End to End Observability

Deployment Status States

- Source Change: Commit to Repo
- Build:
- Container Updated
  - Triggered by deployment update
  - Creates status graph based on topology of deployment
- Event Sent
  - Deployment message sent to EventStream
- Committed to Git
  - GitOps provider commits updates to Git
- Per Assignment
  - Deployed (deployed GitOps commit, consumed deployment message)

# Developer Experience

## Platform Engineer

// Login to AAD
$ titan login

// Create host
$ titan create host --name tim-pi --labels name=tim-pi,platform=kubernetes

// Create a template
$ titan create template device-service -f ./device-service

## Developer Engineer

// Login to AAD
$ titan login

// Show workspaces I have access to, including the default one
$ titan get workspaces

// Register workload (eventually setup CI/CD for secure build)
$ titan create workload temperature-sensor --repo https://github.com/timfpark/temp-sensor [--workspace iot]

// Create a target
$ titan create target tim-pi --labels name=tim-pi

// Create a deployment for our workload
$ titan create deployment prod --workload temperature-sensor --template device-service --target tim-pi
