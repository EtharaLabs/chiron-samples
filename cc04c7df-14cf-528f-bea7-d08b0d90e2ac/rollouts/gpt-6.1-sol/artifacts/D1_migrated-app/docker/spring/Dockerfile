FROM eclipse-temurin:17-jre
WORKDIR /app
COPY folding-stats-service/target/folding-stats-service-1.0.0-SNAPSHOT.jar service.jar
EXPOSE 8080
USER 10001
ENTRYPOINT ["java", "-jar", "/app/service.jar"]
