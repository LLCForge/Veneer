package io.github.llcforge.veneer;

/** Raised for invalid input or unsatisfiable constraints. */
public final class VeneerException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  public VeneerException(String message) {
    super(message);
  }

  public VeneerException(String message, Throwable cause) {
    super(message, cause);
  }
}
