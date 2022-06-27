# akira

Let's do a walkthrough

## Install CLI

One line instructions to install the CLI

## Login

```
$ akira login timfpark@gmail.com
```

Logs in with Github
Creates PAT for CLI and stores it locally?
Automatically creates a group for the user and makes it the default

## Seed sample node.js application

Let's use a sample node.js application to see how easy it is to deploy. To do this, we need a service, so let's template out a quick one using our CLI:

```
$ akira service init hello-service --seed microsoft/nodejs-service-api
```

This is just a convience function and we could have templated it from GitHub itself. Show that it created a repo called hello-service in my Github account and clones it to the

## Seed walkthrough

- Prints "Hello {name}" in response to incoming query.
- Includes Github Action to build container
- Includes metrics and logging

## Deploy node.js application

```
$ akira service create hello-service --template external-service
```

This registers this service with `akira`, specifying that we would like deployments of this service to, by default, use the `external-service` deployment template.

By default, it uses your user group for this service, but alternatively you can use `--group {group}` to specify the group to use for the service.

It also creates an .akira/workload.yaml and adds details about this service (name, group, deployment template).

`akira` enables you to make multiple deployments of your service, so let's make our first one now to deploy the application:

```
$ akira deployment create main --target eastus
```

Here we are creating a `main` deployment and saying that we would like it hosted in on a host matching `eastus`.

`[--service hello-service]` `[--group timfpark]` is assumed in the above because of you are running the command in the `hello-service` directory and Akira will pull defaults from `.akira/service.yaml`.

Behind the scenes this will create a deployment for the service and places it by default at `main.hello-world.timfpark.akira.network`. This is a specific example of the default form `{deployment}.{service}.{group}.akira.network`.

## Fetching Metrics and Logs from Production

## Container Promotion

For production deployments you don't want the build of a container to immediately be pressed into production service.

Instead, these containers are usually promoted in a copy exactly manner (no new container build) from one of the other environments to production.

Let's first create a `prod` deployment for our workload:

```
$ akira deployment create prod --target eastus
```

And then we can promote our `main` development build to production with:

```
$ akira deployment promote main prod
```

This copies the image tag from `main` deployment and applies it as config to the `prod` deployment, deploying it in `prod`.
