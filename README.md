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
$ akira app create hello-service --template external-service --add-action
```

This registers this application with Akira, telling it that it is an external-service
It also adds a Github Action that builds service to container and pushes that to Github Container Registry
It also creates an .akira/workload.yaml and adds details about this workload (name, deployment template)
Let's push that to build our first container.

```
$ git commit
$ git push
```

With our app onboarded, let's make our first deployment:

```
$ akira deployment create main --target eastus
```

[--workload hello-service] is assumed in the above because of where you are running the command.  akira will 
pull the default from .akira/workload.yaml

Here we are creating a `main` deployment and saying that we would like it hosted in the `eastus`
Behind the scenes this will create a deployment for the application and places it at main.hello-world.timfpark.akira.cloud
Pushing the container above will automatically create a `main` branch container and create configuration on `main` that this is the latest image.
