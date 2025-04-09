# pixi-service-generator
A service generator for pixi global packages
Generally speaking, services are just a specific type of CLI application which is
long running, and does so in the background, not under the direct control of a login session.

nginix : https://github.com/conda-forge/nginx-feedstock
solr : https://github.com/conda-forge/apache-solr-feedstock
mysql : https://github.com/conda-forge/mysql-feedstock
mongo : https://github.com/conda-forge/mongodb-feedstock
redis : https://github.com/conda-forge/redis-py-feedstock

I would expect these packages to come with their own unit files.
The Freva package uses serveral of these services and adds systemd unit files for them.
freva : https://github.com/conda-forge/freva-rest-server-feedstock/blob/main/recipe/post-link.sh

I do not see many special accommodation for supporting services via, systemd.
It may be possible to include systemd unit files using the following package:
https://github.com/conda-forge/systemd-feedstock
