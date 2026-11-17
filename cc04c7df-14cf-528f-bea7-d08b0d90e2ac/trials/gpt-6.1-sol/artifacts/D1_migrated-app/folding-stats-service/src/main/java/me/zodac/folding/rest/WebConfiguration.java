package me.zodac.folding.rest;

import java.util.List;
import java.util.Map;
import javax.servlet.http.HttpServletRequest;
import javax.servlet.http.HttpServletResponse;
import me.zodac.folding.service.api.FoldingStatsCore;
import me.zodac.folding.api.UserAuthenticationResult;
import me.zodac.folding.api.util.EncodingUtils;
import me.zodac.folding.api.state.ReadRequired;
import me.zodac.folding.api.state.WriteRequired;
import me.zodac.folding.state.SystemStateManager;
import org.springframework.context.annotation.Configuration;
import org.springframework.http.converter.HttpMessageConverter;
import org.springframework.http.converter.json.GsonHttpMessageConverter;
import org.springframework.web.method.HandlerMethod;
import org.springframework.web.servlet.HandlerInterceptor;
import org.springframework.web.servlet.config.annotation.InterceptorRegistry;
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer;

@Configuration
public class WebConfiguration implements WebMvcConfigurer {
    // Servlet components are registered by Spring; no annotation scanning is needed.
    @org.springframework.context.annotation.Bean
    public org.springframework.boot.web.server.WebServerFactoryCustomizer<org.springframework.boot.web.embedded.tomcat.TomcatServletWebServerFactory> servletConfiguration() {
        return factory -> factory.addContextCustomizers(context -> {
            context.setIgnoreAnnotations(true);
            context.setMapperContextRootRedirectEnabled(false);
        });
    }
    private final FoldingStatsCore core;
    public WebConfiguration(FoldingStatsCore core,
            org.springframework.beans.factory.ObjectProvider<org.springframework.web.servlet.mvc.method.annotation.RequestMappingHandlerMapping> mappings) {
        this.core = core;
        this.mappings = mappings;
    }
    @Override public void extendMessageConverters(List<HttpMessageConverter<?>> converters) {
        converters.removeIf(c -> c instanceof GsonHttpMessageConverter);
        converters.add(new GsonHttpMessageConverter(me.zodac.folding.rest.util.RestUtilConstants.GSON));
    }
    private final org.springframework.beans.factory.ObjectProvider<org.springframework.web.servlet.mvc.method.annotation.RequestMappingHandlerMapping> mappings;
    @Override public void addInterceptors(InterceptorRegistry registry) {
        registry.addInterceptor(new HandlerInterceptor() {
            @Override public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) throws Exception {
                if (request.getMethod().equals("OPTIONS")) {
                    response.setContentType("text/plain;charset=UTF-8");
                    java.util.Set<String> allowed = new java.util.HashSet<>();
                    mappings.getObject().getHandlerMethods().forEach((mapping, method) -> {
                        if (mapping.getPathPatternsCondition() != null && mapping.getPathPatternsCondition().getMatchingCondition(request) != null) {
                            mapping.getMethodsCondition().getMethods().forEach(verb -> allowed.add(verb.name()));
                        }
                    });
                    if (allowed.contains("GET")) allowed.add("HEAD");
                    allowed.add("OPTIONS");
                    response.getWriter().write(String.join(", ", allowed));
                    return false;
                }
                return true;
            }
        });
        registry.addInterceptor(new HandlerInterceptor() {
            @Override public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) {
                if (!(handler instanceof HandlerMethod)) return true;
                RequiredRoles roles = ((HandlerMethod) handler).getMethodAnnotation(RequiredRoles.class);
                if (roles == null) return true;
                try {
                    String auth = request.getHeader("Authorization");
                    if (EncodingUtils.isInvalidBasicAuthentication(auth)) return reject(response, 401);
                    Map<String,String> decoded = EncodingUtils.decodeBasicAuthentication(auth);
                    UserAuthenticationResult user = core.authenticateSystemUser(decoded.get(EncodingUtils.DECODED_USERNAME_KEY), decoded.get(EncodingUtils.DECODED_PASSWORD_KEY));
                    if (!user.isUserExists() || !user.isPasswordMatch()) return reject(response, 401);
                    for (String permitted : roles.value())
                        for (String actual : user.getUserRoles()) if (permitted.equalsIgnoreCase(actual)) return true;
                    return reject(response, 403);
                } catch (Exception ignored) { return true; }
            }
        });
        registry.addInterceptor(new HandlerInterceptor() {
            @Override public boolean preHandle(HttpServletRequest request, HttpServletResponse response, Object handler) {
                if (!(handler instanceof HandlerMethod)) return true;
                HandlerMethod method = (HandlerMethod) handler;
                if ((method.hasMethodAnnotation(ReadRequired.class) && SystemStateManager.current().isReadBlocked()) ||
                    (method.hasMethodAnnotation(WriteRequired.class) && SystemStateManager.current().isWriteBlocked())) return reject(response, 503);
                return true;
            }
        });
    }
    private static boolean reject(HttpServletResponse response, int status) { response.setStatus(status); return false; }

}
