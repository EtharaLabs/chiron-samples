package me.zodac.folding;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.scheduling.annotation.EnableAsync;
import org.springframework.scheduling.annotation.EnableScheduling;
@SpringBootApplication(excludeName = "org.springframework.boot.autoconfigure.jooq.JooqAutoConfiguration")
@EnableAsync(proxyTargetClass = true)
@EnableScheduling
public class FoldingApplication {
    public static void main(String[] args) { SpringApplication.run(FoldingApplication.class, args); }
}
