
To get your setup ready for the demo, there are a few changes you will have to
make to your infrastructure to make sure we correctly evaluate your setup:

1. The first change involves how you notify the `notifications-service`.

   When performing a request to the `notifications-service` you should add a
   new query parameter that is used to identify your group. For this purpose, a
   new file named `token` has been added to your group's one drive credentials
   folder. The contents of this file, is the value of the `token` query
   parameter you have to pass when notifying the `notifications-service`.

   Additionally, you also have to change the host to which you are sending the
   request to `notifications-service.cc2023.4400app.me`.

   As such, when performing a request to the notifications-service, the url
   should look as follows:
   `https://notifications-service.cc2023.4400app.me/api/notify?token=<your-token>`.

   **You can keep sending the body as you were.**
1. Your REST API should be reachable at `<your-vm-ip>:3003`. 

   This implies that when querying your REST API, the endpoints will be: 
   - Temperature:
     `http://<your-vm-ip>:3003/temperature?experiment-id=<experiment-id>&start-time=<start-time>&end-time=<end-time>`
   - Out-of-bounds:
     `http://<your-vm-ip>:3003/temperature/out-of-bounds?experiment-id=<experiment-id>`
1. Your prometheus instance should be reachable at `<your-vm-ip>:3008`. Make
   sure that your prometheus instance is exposing the metrics it is scraping
   from the `node_exporter`.

   > Note: you only need the prometheus service from the local setup
   > `docker-compose` file running, i.e., you don't need to have the grafana
   > service running.
1. A grafana instance is provided so you can view the current perceived state
   of your infrastructure.

   Visit `https://grafana.cc2023.4400app.me` and login with the credentials
   provided in the `grafana` file in your group's credentials folder in one
   drive.
1. Have your consumers read from the `experiment` topic instead of your group
   topic. The data produced throughout the demo will go to this topic.

   > Note: Your group client only has read permissions on this topic, therefore
   > you cannot run your producers to insert data into the topic. 
