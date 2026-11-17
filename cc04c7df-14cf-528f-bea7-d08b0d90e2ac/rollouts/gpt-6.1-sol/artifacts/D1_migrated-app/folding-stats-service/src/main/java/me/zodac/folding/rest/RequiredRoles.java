package me.zodac.folding.rest;
import java.lang.annotation.*;
/** Application roles required by a controller operation. */
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface RequiredRoles { String[] value(); }
