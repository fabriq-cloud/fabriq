# akira

## Login

```
$ akira login timfpark@gmail.com
```

Logs in with Github
Creates PAT for CLI and stores it locally?
Automatically creates a user workspace and makes it the default

## Create node.js application

Let's use a sample node.js application to see how easy it is to deploy
Template sample hello world node.js application out from Github
Clone it locally

## Deploy node.js application

```
$ akira workload create hello-service --template external-service --add-action
```

This registers this application with Akira, telling it that it is an external-service
It also adds a Github Action that builds service to container and pushes that to Github Container Registry
It also creates an .akira/workload.yaml and adds details about this workload (name, deployment template).

Our default action builds a container and deploys it automatically, so let's create a `main` deployment so that our deployment works when we push the action:

```
$ akira deployment create main --target eastus
```

Here we are creating a `main` deployment and saying that we would like it hosted in any host matching `eastus`

Behind the scenes this will create a deployment for the application and places it at main.hello-world.timfpark.akira.network

`[--workload hello-service]` `[--workspace timfpark]` is assumed in the above because of where you are running the command. akira will pull the default from .akira/workload.yaml
Let's push our action to build our first container.

```
$ git commit
$ git push
```

This builds a first container for the main branch.

Pushing the container above will automatically create a `main` branch container and will configure the `main`
deployment with the image name once it is built.

These changes will cause an orchestration to be run to deploy the workload and expose it such that we can go to our browser and get a hello.

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

## Fetching Metrics and Logs from Production
