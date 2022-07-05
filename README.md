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
Automatically creates a group for the user and makes it the default?

## Seed sample node.js application

Let's use a sample node.js application to see how easy it is to deploy. To do this, we need a service, so let's template out a quick one using our CLI:

```
$ akira service init hello-service --seed microsoft/nodejs-service-api
```

This is just a convience function and we could have templated it from GitHub itself. Show that it created a repo called hello-service in my Github account and clones it to the

## Seed walkthrough

- Run It
- Show it prints "Hello {name}" in response to incoming query.
- Show it includes Github Action to build container
- Show it includes metrics and logging (v2)

## Deploy node.js application

Let's deploy it. First, we want to register our service:

```
$ akira service create hello-service --template external-service --target eastus
```

This registers this service with `akira`, specifying that we would like deployments of this service to, by default, use the `external-service` deployment template and target hosts matching `eastus`.

By default, it uses your user group for this service, but alternatively you can use `--group {group}` to specify the group to use for the service.

It also creates an .akira/workload.yaml and adds details about this service (name, group, deployment template).

`akira` enables you to make multiple deployments of your service so let's make our first one now:

```
$ akira deployment create
deployment created:
   name: main (default from git branch)
   service: hello-service
   template: external-service (inherited from service)
   target: eastus (inherited from service)
   group: timfpark
```

We could name this deployment with a `name` parameter, but by default `akira` will choose the name of the current branch of our Git repo.

`[--service hello-service]` is assumed in the above because of you are running the command in the `hello-service` service repo and Akira will pull defaults from `.akira/service.yaml`.

Likewise, since we didn't override them, the deployment will inherit the same deployment template and target from the service. This is usually what you want, but if you can override them, if, for example you have a very large production deployment or very small dev deployment that you want to do.

Behind the scenes this will create a deployment for the service, will match it to a host that matches our `eastus` target, and because we used an `external-service` deployment template, will surface it by default at `main.hello-world.timfpark.akira.network`. This is a specific example of the default form `{deployment}.{service}.{group}.akira.network`.

Additionally, each time that we push a commit to our `main` branch, our GitHub CI will build our service, update our `main` deployment.

## Container Promotion

For production deployments you don't want the build of a container to immediately be deployed. Instead service teams typically test a build in another environment and then promote it in a copy exactly manner (no new container build) to production.

Let's first create a `prod` deployment for our workload:

```
$ akira deployment create prod
deployment created:
   name: prod
   service: hello-service
   template: external-service (inherited from service)
   target: eastus (inherited from service)
   group: timfpark
```

In this case we are not letting Akira default to the name of our current branch, but instead specifying `prod` explicitly.

And then we can promote our `main` development build to production with:

```
$ akira deployment promote main prod
```

This copies the image tag from `main` deployment and applies it as config to the `prod` deployment, deploying it in `prod`.
